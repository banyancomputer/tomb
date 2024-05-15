use crate::{
    cli::{
        commands::{
            drives::{
                local::{LocalBanyanFS, LocalLoadedDrive},
                sync::SyncLoadedDrive,
            },
            RunnableCommand,
        },
        specifiers::{DriveId, DriveSpecifier},
    },
    drive::*,
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        local_share::DriveAndKeyId,
        OnDisk,
    },
    utils::name_of,
    ConfigStateError, NativeError,
};
use async_trait::async_trait;
use banyanfs::{api::platform, codec::crypto::SigningKey, filesystem::Drive};

use clap::Subcommand;
use colored::Colorize;
use std::{env::current_dir, path::PathBuf};
use tracing::{info, warn};

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DrivesCommand {
    /// List all Drives
    Ls,
    /// Initialize a new Drive
    Create {
        /// Drive Root
        #[arg(short, long)]
        origin: Option<PathBuf>,
    },
    /// Prepare a Drive for Pushing by encrypting new data
    Prepare {
        /// Drive in question
        #[clap(flatten)]
        ds: DriveSpecifier,

        /// Follow symbolic links
        #[arg(short, long)]
        follow_links: bool,
    },
    /// Reconstruct a Drive filesystem locally
    Restore(DriveSpecifier),
    /// Delete a Drive
    Delete(DriveSpecifier),
    /// Sync Drive data to or from remote
    Sync(DriveSpecifier),
    /*
    /// Drive info
    Info(DriveSpecifier),
    /// Drive data usage
    Usage(DriveSpecifier),
    /// Drive Key management
    Keys {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: KeyCommand,
    },
    */
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DrivesCommand {
    async fn run_internal(self) -> Result<(), NativeError> {
        let mut global = GlobalConfig::decode(&GlobalConfigId).await?;

        match self {
            // List all Buckets tracked remotely and locally
            DrivesCommand::Ls => {
                let remote_drives = match global.api_client().await {
                    Ok(client) => platform::drives::get_all(&client).await?,
                    Err(_) => {
                        warn!("You aren't logged in. Login to see remote drives.");
                        vec![]
                    }
                };

                let user_key_id = global.selected_user_key_id()?;
                let local_drive_names = Drive::entries()?;
                for name in local_drive_names.iter() {
                    let id = &DriveAndKeyId {
                        drive_id: name.clone(),
                        user_key_id: user_key_id.clone(),
                    };
                    let unlocked = Drive::decode(&id).await.is_ok();
                    let origin = global
                        .get_origin(&name)
                        .map(|p| p.display().to_string())
                        .unwrap_or("Unknown".to_string());

                    if let Some(_remote) = remote_drives.iter().find(|r| r.name == *name) {
                        info!(name, origin, ?unlocked, "Sync Drive");
                    } else {
                        info!(name, origin, ?unlocked, "Local Drive");
                    }
                }

                for remote in remote_drives
                    .into_iter()
                    .filter(|r| !local_drive_names.contains(&r.name))
                {
                    info!(
                        name = remote.name,
                        origin = "None",
                        unlocked = false,
                        "Remote Drive"
                    );
                }

                Ok(())
            }
            // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
            DrivesCommand::Create { origin } => {
                let origin = origin.unwrap_or(current_dir()?);
                let drive_id =
                    name_of(&origin).ok_or(ConfigStateError::ExpectedPath(origin.clone()))?;
                // Save location association
                global.set_origin(&drive_id, &origin);
                global.encode(&GlobalConfigId).await?;
                let user_key_id = global.selected_user_key_id()?;
                let id = DriveAndKeyId {
                    drive_id: drive_id.clone(),
                    user_key_id: user_key_id.clone(),
                };
                // Create and encode the Drive and Store
                let lbfs = LocalBanyanFS::init(&id).await?;
                info!("<< CREATED LOCAL DRIVE >>");

                if let Ok(client) = global.api_client().await {
                    let public_key = SigningKey::decode(&user_key_id).await?.verifying_key();
                    let remote_id =
                        platform::drives::create(&client, &drive_id, &public_key).await?;
                    info!("<< CREATED REMOTE DRIVE >>");
                }

                info!(
                    "{}\n{:?}",
                    "<< NEW DRIVE CREATED >>".green(),
                    lbfs.drive.id()
                );
                Ok(())
            }
            DrivesCommand::Prepare {
                ds,
                follow_links: _,
            } => {
                //let mut ld = SyncLoadedDrive::load(&ds.into(), &global).await?;
                let mut ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                operations::prepare(&mut ld.lbfs.drive, &mut ld.lbfs.store, &ld.origin).await?;
                ld.lbfs.encode(&ld.id).await?;
                info!(
                    "{}\n{:?}",
                    "<< DRIVE DATA STORED SUCCESSFULLY >>".green(),
                    ld.lbfs.drive.id()
                );
                Ok(())
            }
            DrivesCommand::Delete(ds) => {
                let ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                global.remove_origin(&ld.id.drive_id)?;
                Drive::erase(&ld.id).await?;
                LocalBanyanFS::erase(&ld.id).await?;
                global.encode(&GlobalConfigId).await?;

                info!(
                    "{}\n{:?}",
                    "<< DRIVE DATA DELETED SUCCESSFULLY >>".green(),
                    ld.lbfs.drive.id()
                );
                Ok(())
            }
            DrivesCommand::Restore(ds) => {
                //let client = global.api_client().await?;
                //let drive = platform::drives::get(&client, drive_id).await?;

                let mut ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                operations::restore(&mut ld.lbfs.drive, &mut ld.lbfs.store, &ld.origin).await?;
                info!(
                    "{}\n{:?}",
                    "<< DRIVE DATA RESTORED TO DISK SUCCESSFULLY >>".green(),
                    ld.lbfs.drive.id()
                );
                Ok(())
            }
            DrivesCommand::Sync(ds) => {
                let client = global.api_client().await?;
                let remote_drives = platform::drives::get_all(&client).await?;
                let di: DriveId = ds.into();
                //if let Ok(drive_id) = di.get_id().await { }

                //let remote = if let DriveId::DriveId(id) = di { }
                // There is already a local drive here
                //if let Ok(_ld) = LoadedDrive::load(&di, &global).await {}
                todo!()
            }
        }
    }
}
