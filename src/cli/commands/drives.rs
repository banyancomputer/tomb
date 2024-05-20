use crate::{
    cli::{
        commands::{
            drives::local::{LocalBanyanFS, LocalLoadedDrive},
            RunnableCommand,
        },
        display::Persistence,
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
use cli_table::{print_stdout, Cell, Table};
use tokio::fs::rename;

use std::{env::current_dir, path::PathBuf};
use tracing::{info, warn};

use super::drive_access::DriveAccessCommand;

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DrivesCommand {
    /// List all Drives
    Ls,
    /// Initialize a new Drive
    Create {
        /// Drive Root
        #[arg(short, long)]
        path: Option<PathBuf>,
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
    /// Change the name of a Drive
    Rename {
        /// Drive in question
        #[clap(flatten)]
        ds: DriveSpecifier,

        /// New name
        new_name: String,
    },
    /// Drive Key management
    Access {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: DriveAccessCommand,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DrivesCommand {
    async fn run_internal(self) -> Result<(), NativeError> {
        let mut global = GlobalConfig::decode(&GlobalConfigId).await?;
        use DrivesCommand::*;
        match self {
            // List all Buckets tracked remotely and locally
            Ls => {
                let remote_drives = match global.get_client().await {
                    Ok(client) => platform::drives::get_all(&client).await?,
                    Err(_) => {
                        warn!("You aren't logged in. Login to see remote drives.");
                        vec![]
                    }
                };

                let mut table_rows = Vec::new();
                let local_drive_names = Drive::entries();

                // Find the drives that exist both locally and remotely
                let mut sync_names = Vec::new();
                for local_name in local_drive_names.iter() {
                    for remote in remote_drives.iter() {
                        if *local_name == remote.name {
                            sync_names.push(local_name.clone());
                            table_rows.push(vec![
                                remote.name.clone().cell(),
                                remote.id.clone().cell(),
                                global.get_path(local_name)?.display().cell(),
                                Persistence::Sync.cell(),
                            ])
                        }
                    }
                }

                for local_name in local_drive_names.into_iter() {
                    if !sync_names.contains(&local_name) {
                        table_rows.push(vec![
                            local_name.clone().cell(),
                            "N/A".cell(),
                            global.get_path(&local_name)?.display().cell(),
                            Persistence::LocalOnly.cell(),
                        ]);
                    }
                }

                for remote in remote_drives.into_iter() {
                    if !sync_names.contains(&remote.name) {
                        table_rows.push(vec![
                            remote.name.clone().cell(),
                            remote.id.clone().cell(),
                            "N/A".cell(),
                            Persistence::RemoteOnly.cell(),
                        ])
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
            // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
            Create { path } => {
                let path = path.unwrap_or(current_dir()?);
                let drive_id =
                    name_of(&path).ok_or(ConfigStateError::ExpectedPath(path.clone()))?;
                // Save location association
                global.set_path(&drive_id, &path);
                global.encode(&GlobalConfigId).await?;
                let user_key_id = global.selected_user_key_id()?;
                let id = DriveAndKeyId::new(&drive_id, &user_key_id);

                if let Ok(client) = global.get_client().await {
                    let public_key = SigningKey::decode(&user_key_id).await?.verifying_key();
                    let _remote_id =
                        platform::drives::create(&client, &drive_id, &public_key).await?;
                    info!("<< CREATED REMOTE DRIVE >>");
                } else {
                    // Create and encode the Drive and Store
                    LocalBanyanFS::init(&id).await?;
                    info!("<< CREATED LOCAL DRIVE >>");
                }

                Ok(())
            }
            Prepare {
                ds,
                follow_links: _,
            } => {
                let mut ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                //let mut ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                operations::prepare(&mut ld.bfs.drive, &mut ld.bfs.store, &ld.path).await?;
                ld.bfs.encode(&ld.id).await?;
                info!("<< DRIVE DATA STORED SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());
                Ok(())
            }
            Delete(ds) => {
                let ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                global.remove_path(&ld.id.drive_id)?;
                Drive::erase(&ld.id).await?;
                LocalBanyanFS::erase(&ld.id).await?;
                global.encode(&GlobalConfigId).await?;

                info!("<< DRIVE DATA DELETED SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());

                Ok(())
            }
            Restore(ds) => {
                //let client = global.api_client().await?;
                //let drive = platform::drives::get(&client, drive_id).await?;

                let mut ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                operations::restore(&mut ld.bfs.drive, &mut ld.bfs.store, &ld.path).await?;
                info!("<< DRIVE DATA RESTORED TO DISK SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());
                Ok(())
            }
            Sync(ds) => {
                let client = global.get_client().await?;
                let _remote_drives = platform::drives::get_all(&client).await?;
                let _di: DriveId = ds.into();
                //if let Ok(drive_id) = di.get_id().await { }

                //let remote = if let DriveId::DriveId(id) = di { }
                // There is already a local drive here
                //if let Ok(_ld) = LoadedDrive::load(&di, &global).await {}
                todo!()
            }
            Rename { ds, new_name } => {
                let di = DriveId::from(ds);
                let loaded = LocalLoadedDrive::load(&di, &global).await?;

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
            }
            Access { subcommand } => subcommand.run_internal().await,
        }
    }
}
