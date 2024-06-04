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
    ConfigStateError, NativeError,
};
use async_trait::async_trait;
use banyanfs::{
    api::platform,
    codec::{crypto::SigningKey, header::AccessMaskBuilder},
    filesystem::{Drive, DriveLoader},
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
    /// Sync Drive data to or from remote
    Sync,
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

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DriveOperationCommand {
    type Payload = DriveOperationPayload;
    async fn run_internal(self, payload: Self::Payload) -> Result<(), NativeError> {
        use DriveOperationCommand::*;
        let mut global = GlobalConfig::decode(&GlobalConfigId).await?;
        match self {
            // Info
            Info => {
                info!("trying local");
                let mut table_rows = Vec::new();
                let api = helpers::api_drive_with_name(&global, &payload.id.drive_id).await;
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
                let mut ld = LocalLoadedDrive::load(&payload).await?;
                info!("loaded!");
                operations::prepare(&mut ld.bfs.drive, &mut ld.bfs.store, &ld.path).await?;
                ld.bfs.encode(&ld.id).await?;
                info!("<< DRIVE DATA STORED SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());
                Ok(())
            }
            Delete => {
                let ld = LocalLoadedDrive::load(&payload).await?;
                global.remove_path(&ld.id.drive_id)?;
                Drive::erase(&ld.id).await?;
                LocalBanyanFS::erase(&ld.id).await?;
                global.encode(&GlobalConfigId).await?;

                info!("<< DRIVE DATA DELETED SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());

                Ok(())
            }
            Restore => {
                //let client = global.api_client().await?;
                //let drive = platform::drives::get(&client, drive_id).await?;

                let mut ld = LocalLoadedDrive::load(&payload).await?;
                operations::restore(&mut ld.bfs.drive, &mut ld.bfs.store, &ld.path).await?;
                info!("<< DRIVE DATA RESTORED TO DISK SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());
                Ok(())
            }
            Sync => {
                let client = global.get_client().await?;
                let remote_drives = platform::drives::get_all(&client).await?;

                // Get the remote drive, creating it if need be
                let remote_drive = match remote_drives
                    .into_iter()
                    .find(|remote_drive| remote_drive.name == payload.id.drive_id)
                {
                    Some(remote_drive) => remote_drive,
                    None => {
                        warn!("Remote drive was missing, creating it!");
                        let remote_drive_id =
                            platform::drives::create(&client, &payload.id.drive_id).await?;
                        platform::drives::get(&client, &remote_drive_id).await?
                    }
                };

                info!("found the remote");

                // If there is already a drive stored on disk
                if let Ok(local_drive) = LocalBanyanFS::decode(&payload.id).await {
                    // Sync the drive
                    local_drive.sync(&remote_drive.id).await?;
                }
                // If we need to pull down
                else {
                    // We need the key loaded
                    let user_key = SigningKey::decode(&payload.id.user_key_id).await?;

                    let current_metadata =
                        platform::metadata::get_current(&client, &remote_drive.id).await?;
                    let metadata_id = current_metadata.id();

                    // metadata for a drive (if we've seen zero its safe to create a new drive, its not otherwise).
                    let mut stream =
                        platform::metadata::pull_stream(&client, &remote_drive.id, &metadata_id)
                            .await?;
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
                    OnDisk::encode(&drive, &payload.id).await?;

                    // Create the location where reconstructed files will be at home
                    let files_dir = PathBuf::from(format!("{}/banyan", env!("HOME")))
                        .join(&payload.id.drive_id);
                    create_dir_all(&files_dir).await?;
                    global.set_path(&payload.id.drive_id, &files_dir);
                    global.encode(&GlobalConfigId).await?;

                    LocalBanyanFS::init_from_drive(&payload.id, drive).await?;
                }

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

                global.remove_path(&old_id.drive_id)?;
                global.set_path(&new_id.drive_id, &new_path);
                global.encode(&GlobalConfigId).await?;

                info!("<< RENAMED DRIVE LOCALLY >>");

                if let Ok(drive_platform_id) = global.drive_platform_id(&old_id.drive_id).await {
                    let client = global.get_client().await?;
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
