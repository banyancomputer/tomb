use std::path::PathBuf;

use banyanfs::{
    api::{
        platform::{self, ApiDrive},
        VecStream,
    },
    codec::{
        crypto::{SigningKey, VerifyingKey},
        header::{AccessMaskBuilder, ContentOptions},
    },
    filesystem::DriveLoader,
    stores::{SyncTracker, SyncableDataStore},
    utils::crypto_rng,
};
use futures::{io::Cursor, StreamExt};
use tokio::fs::create_dir_all;
use tracing::{error, info, warn};

use crate::{
    cli::commands::drives::CborSyncTracker,
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        local_share::DriveAndKeyId,
        OnDisk,
    },
    utils::prompt_for_bool,
    NativeError,
};

use super::LocalBanyanFS;

pub async fn api_drive_with_name(global: &GlobalConfig, name: &str) -> Option<ApiDrive> {
    api_drives(global)
        .await
        .into_iter()
        .find(|api| api.name == name)
}

pub async fn api_drives(global: &GlobalConfig) -> Vec<ApiDrive> {
    match global.get_client().await {
        Ok(client) => match platform::drives::get_all(&client).await {
            Ok(d) => d,
            Err(err) => {
                error!("Logged in, but failed to fetch remote drives. {err}");
                vec![]
            }
        },
        Err(_) => {
            warn!("You aren't logged in. Login to see remote drives.");
            vec![]
        }
    }
}

pub async fn sync(id: &DriveAndKeyId, global: &mut GlobalConfig) -> Result<(), NativeError> {
    let client = global.get_client().await?;

    // Get the remote drive, creating it if need be
    let api_drive = match api_drive_with_name(&global, &id.drive_id).await {
        Some(api_drive) => api_drive,
        None => {
            if prompt_for_bool("No remote drive with this name. Create one?", 'y', 'n') {
                let remote_drive_id = platform::drives::create(&client, &id.drive_id).await?;
                platform::drives::get(&client, &remote_drive_id).await?
            } else {
                error!("Cannot sync when no remote drive matches query.");
                return Ok(());
            }
        }
    };

    // If we need to push up
    if let Ok(mut local_drive) = LocalBanyanFS::decode(&id).await {
        let bucket_id = api_drive.id;
        // Sync the drive
        let mut store = local_drive.go_online().await?;

        let mut rng = crypto_rng();
        // For Metadata push
        let content_options = ContentOptions::metadata();
        let mut encoded_drive = Vec::new();
        local_drive
            .drive
            .encode(&mut rng, content_options, &mut encoded_drive)
            .await?;
        let expected_data_size = store.unsynced_data_size().await?;
        let root_cid = local_drive.drive.root_cid().await?;

        let verifying_keys: Vec<VerifyingKey> = local_drive
            .drive
            .verifying_keys()
            .await
            .into_iter()
            .filter_map(|(key, mask)| {
                if !mask.is_historical() {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        let deleted_block_cids = store.deleted_cids().await?;
        let drive_stream = VecStream::new(encoded_drive).pinned();

        let push_response = platform::metadata::push_stream(
            &client,
            &bucket_id,
            expected_data_size,
            root_cid,
            //last_saved_metadata.as_ref().map(|m| m.id()).clone(),
            None,
            drive_stream,
            verifying_keys,
            deleted_block_cids,
        )
        .await?;
        let new_metadata_id = push_response.id();

        if let Some(host) = push_response.storage_host() {
            if let Err(err) = store.set_sync_host(host.clone()).await {
                // In practice this should never happen, the trait defines an error type for
                // flexibility in the future but no implementations currently produce an error.
                warn!("failed to set sync host: {err}");
            }
            if let Some(grant) = push_response.storage_authorization() {
                client.record_storage_grant(host, grant).await;
            }
        }

        let _new_metadata = platform::metadata::get(&client, &bucket_id, &new_metadata_id).await?;
        match store.sync(&new_metadata_id).await {
            Ok(()) => {
                info!("<< SYNCED DRIVE DATA TO PLATFORM >>");
                // Empty the tracker because it worked
                local_drive.tracker = CborSyncTracker::default();
            }
            Err(err) => {
                warn!("failed to sync data store to remotes, data remains cached locally but unsynced and can be retried: {err}");
            }
        }
        local_drive.encode(&id).await?;
    }
    // If we need to pull down
    else {
        // We need the key loaded
        let user_key = SigningKey::decode(&id.user_key_id).await?;

        let current_metadata = platform::metadata::get_current(&client, &api_drive.id).await?;
        let metadata_id = current_metadata.id();

        // metadata for a drive (if we've seen zero its safe to create a new drive, its not otherwise).
        let mut stream =
            platform::metadata::pull_stream(&client, &api_drive.id, &metadata_id).await?;
        let mut drive_bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk
                .map_err(|e| NativeError::Custom(format!("{e}")))?
                .to_vec();
            drive_bytes.extend(bytes);
        }

        let mut drive_cursor = Cursor::new(drive_bytes);
        let drive_loader = DriveLoader::new(&user_key);
        let drive = drive_loader.from_reader(&mut drive_cursor).await?;

        // Ensure the platform key is present
        let platform_key = client.platform_public_key().await?;
        if !drive.has_maintenance_access(&platform_key.actor_id()).await {
            let access_mask = AccessMaskBuilder::maintenance().protected().build()?;
            drive
                .authorize_key(&mut crypto_rng(), platform_key, access_mask)
                .await?;
        }

        // Encode Drive
        OnDisk::encode(&drive, &id).await?;

        // Create the location where reconstructed files will be at home
        let files_dir = PathBuf::from(format!("{}/banyan", env!("HOME"))).join(&id.drive_id);
        create_dir_all(&files_dir).await?;
        global.set_path(&id.drive_id, &files_dir);
        global.encode(&GlobalConfigId).await?;

        LocalBanyanFS::init_from_drive(&id, drive).await?;
    }
    Ok(())
}
