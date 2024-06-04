use std::path::PathBuf;

use banyanfs::{
    api::platform::{self, ApiDrive},
    codec::{crypto::SigningKey, header::AccessMaskBuilder},
    filesystem::DriveLoader,
    utils::crypto_rng,
};
use futures::{io::Cursor, StreamExt};
use tokio::fs::create_dir_all;
use tracing::{error, warn};

use crate::{
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        local_share::DriveAndKeyId,
        OnDisk,
    },
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

pub async fn sync(mut global: GlobalConfig, id: &DriveAndKeyId) -> Result<(), NativeError> {
    let client = global.get_client().await?;

    // Get the remote drive, creating it if need be
    let api_drive = match api_drive_with_name(&global, &id.drive_id).await {
        Some(api_drive) => api_drive,
        None => {
            warn!("Remote drive was missing, creating it!");
            let remote_drive_id = platform::drives::create(&client, &id.drive_id).await?;
            platform::drives::get(&client, &remote_drive_id).await?
        }
    };

    // If there is already a drive stored on disk
    if let Ok(local_drive) = LocalBanyanFS::decode(&id).await {
        // Sync the drive
        local_drive.sync(&api_drive.id).await?;
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
