use std::path::PathBuf;

use crate::{
    cli::{
        commands::{
            drives::{LocalBanyanFS, LocalLoadedDrive},
            helpers,
        },
        display::Persistence,
        RunnableCommand,
    },
    drive::operations,
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        local_share::DriveAndKeyId,
        OnDisk,
    },
    utils::prompt_for_bool,
    ConfigStateError, NativeError,
};
use async_trait::async_trait;
use banyanfs::{
    api::platform,
    codec::{crypto::SigningKey, header::AccessMaskBuilder},
    filesystem::{Drive, DriveLoader},
    stores::MemorySyncTracker,
    utils::crypto_rng,
};
use clap::Subcommand;
use cli_table::{print_stdout, Cell, Table};
use futures::{io::Cursor, StreamExt};
use tokio::fs::{create_dir_all, rename};
use tracing::*;

#[derive(Subcommand, Clone, Debug)]
pub enum DriveOperationCommand {
    Info,
    /// Prepare a Drive for Pushing by encrypting new data
    Prepare {
        /// Follow symbolic links
        #[arg(short, long)]
        follow_links: bool,
    },
    /// Reconstruct a Drive filesystem locally
    Restore,
    /// Delete a Drive
    Delete,
    /// Change the name of a Drive
    Rename {
        new_name: String,
    }, //(String),
       /*
       /// Drive Key management
       Access {
           /// Subcommand
           #[clap(subcommand)]
           subcommand: DriveAccessCommand,
       },
       */
}

#[derive(Debug, Clone)]
pub struct DriveOperationPayload {
    pub id: DriveAndKeyId,
    pub global: GlobalConfig,
}

impl DriveOperationPayload {
    pub async fn sync(&mut self) -> Result<(), NativeError> {
        let client = self.global.get_client().await?;

        // Get the remote drive, creating it if need be
        let api_drive = match helpers::api_drive_with_name(&self.global, &self.id.drive_id).await {
            Some(api_drive) => api_drive,
            None => {
                if prompt_for_bool("No remote drive with this name. Create one?", 'y', 'n') {
                    let remote_drive_id =
                        platform::drives::create(&client, &self.id.drive_id).await?;
                    platform::drives::get(&client, &remote_drive_id).await?
                } else {
                    error!("Cannot sync when no remote drive matches query.");
                    return Ok(());
                }
            }
        };

        // If there is already a drive stored on disk
        if let Ok(local_drive) = LocalBanyanFS::decode(&self.id).await {
            // Sync the drive
            local_drive.sync(&api_drive.id).await?;
        }
        // If we need to pull down
        else {
            // We need the key loaded
            let user_key = SigningKey::decode(&self.id.user_key_id).await?;

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
            OnDisk::encode(&drive, &self.id).await?;

            // Create the location where reconstructed files will be at home
            let files_dir =
                PathBuf::from(format!("{}/banyan", env!("HOME"))).join(&self.id.drive_id);
            create_dir_all(&files_dir).await?;
            self.global.set_path(&self.id.drive_id, &files_dir);
            self.global.encode(&GlobalConfigId).await?;

            LocalBanyanFS::init_from_drive(&self.id, drive).await?;
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DriveOperationCommand {
    type Payload = DriveOperationPayload;
    async fn run_internal(self, mut payload: Self::Payload) -> Result<(), NativeError> {
        use DriveOperationCommand::*;
        match self {
            // Info
            Info => {
                let mut table_rows = Vec::new();
                let api = helpers::api_drive_with_name(&payload.global, &payload.id.drive_id).await;
                let local = LocalLoadedDrive::load(&payload).await.ok();
                match (api, local) {
                    (Some(api), Some(local)) => table_rows.push(vec![
                        api.name.clone().cell(),
                        api.id.clone().cell(),
                        local.path.display().cell(),
                        Persistence::Sync.cell(),
                    ]),
                    (Some(api), None) => table_rows.push(vec![
                        api.name.clone().cell(),
                        api.id.clone().cell(),
                        "N/A".cell(),
                        Persistence::RemoteOnly.cell(),
                    ]),
                    (None, Some(local)) => table_rows.push(vec![
                        local.id.drive_id.cell(),
                        "N/A".cell(),
                        local.path.display().cell(),
                        Persistence::RemoteOnly.cell(),
                    ]),
                    (None, None) => {
                        return Err(ConfigStateError::MissingDrive(payload.id.drive_id).into());
                    }
                }

                let table = table_rows.table().title(vec![
                    "Name".cell(),
                    "ID".cell(),
                    "Path".cell(),
                    "Persistence".cell(),
                ]);
                print_stdout(table)?;
                Ok(())
            }
            Prepare { follow_links: _ } => {
                payload.sync().await?;
                let mut ld = LocalLoadedDrive::load(&payload).await?;
                operations::prepare(&mut ld.bfs.drive, &mut ld.bfs.store, &ld.path).await?;

                // If we can see an api drive
                if let Some(api_drive) =
                    helpers::api_drive_with_name(&payload.global, &ld.id.drive_id).await
                {
                    // Sync and create a clean slate in the tracker
                    ld.bfs.sync(&api_drive.id).await?;
                    ld.bfs.tracker = MemorySyncTracker::default();
                }

                // Encode
                ld.bfs.encode(&ld.id).await?;
                info!("<< DRIVE DATA STORED SUCCESSFULLY >>");
                Ok(())
            }
            Delete => {
                let ld = LocalLoadedDrive::load(&payload).await?;
                payload.global.remove_path(&ld.id.drive_id)?;
                Drive::erase(&ld.id).await?;
                LocalBanyanFS::erase(&ld.id).await?;
                payload.global.encode(&GlobalConfigId).await?;

                info!("<< DRIVE DATA DELETED SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());

                Ok(())
            }
            Restore => {
                payload.sync().await?;
                let mut ld = LocalLoadedDrive::load(&payload).await?;
                match ld.bfs.go_online().await {
                    Ok(mut store) => {
                        info!("Utilizing API sync for this restoration");
                        operations::restore(&mut ld.bfs.drive, &mut store, &ld.path).await?;
                    }
                    Err(_) => {
                        warn!("Unable to go online. Restoration will fail if data is not already on disk.");
                        operations::restore(&mut ld.bfs.drive, &mut ld.bfs.store, &ld.path).await?;
                    }
                }
                info!("<< DRIVE DATA RESTORED TO DISK SUCCESSFULLY >>");
                Ok(())
            }
            Rename { new_name } => {
                let loaded = LocalLoadedDrive::load(&payload).await?;
                let old_id = loaded.id.clone();
                let new_id = DriveAndKeyId::new(&new_name, &old_id.user_key_id);

                // Rename drive.bfs
                Drive::rename(&old_id, &new_id).await?;
                // Rename drive_blocks folder
                LocalBanyanFS::rename(&old_id, &new_id).await?;

                // Rename the folder in user land
                let old_path = loaded.path.clone();
                let new_path = old_path.parent().unwrap().join(new_name);
                rename(old_path, &new_path).await?;

                payload.global.remove_path(&old_id.drive_id)?;
                payload.global.set_path(&new_id.drive_id, &new_path);
                payload.global.encode(&GlobalConfigId).await?;

                info!("<< RENAMED DRIVE LOCALLY >>");

                if let Ok(drive_platform_id) =
                    payload.global.drive_platform_id(&old_id.drive_id).await
                {
                    let client = payload.global.get_client().await?;
                    platform::drives::update(
                        &client,
                        &drive_platform_id,
                        platform::ApiDriveUpdateAttributes {
                            name: Some(new_id.drive_id),
                        },
                    )
                    .await?;
                    info!("<< RENAMED DRIVE REMOTELY >>");
                }

                Ok(())
            } //Access { subcommand } => subcommand.run_internal(payload).await,
        }
    }
}
