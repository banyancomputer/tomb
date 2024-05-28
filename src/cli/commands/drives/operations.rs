use crate::{
    cli::{
        commands::drives::{LocalBanyanFS, LocalLoadedDrive},
        RunnableCommand,
    },
    drive::operations,
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        local_share::DriveAndKeyId,
        OnDisk,
    },
    NativeError,
};
use async_trait::async_trait;
use banyanfs::{api::platform, filesystem::Drive};
use clap::Subcommand;
use tokio::fs::rename;
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
                let _local = LocalLoadedDrive::load(&payload).await?;
                /*
                info!("local: {:?}", local.path.display());
                let client = global.get_client().await?;
                let platform_id = global.drive_platform_id(&drive_id).await?;
                info!("pid: {:?}", platform_id);
                let platform_drive = platform::drives::get(&client, &drive_id).await?;
                info!("pd: {:?}", platform_drive.id);
                */
                Ok(())
            }
            Prepare { follow_links: _ } => {
                let mut ld = LocalLoadedDrive::load(&payload).await?;
                //let mut ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
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
                let _remote_drives = platform::drives::get_all(&client).await?;
                //if let Ok(drive_id) = di.get_id().await { }

                //let remote = if let DriveId::DriveId(id) = di { }
                // There is already a local drive here
                //if let Ok(_ld) = LoadedDrive::load(&di, &global).await {}
                todo!()
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
