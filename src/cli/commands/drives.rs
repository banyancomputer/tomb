use crate::{
    cli::{
        commands::{
            drives::{
                local::{LocalBanyanFS, LocalLoadedDrive},
                sync::{SyncBanyanFS, SyncLoadedDrive},
            },
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

        match self {
            // List all Buckets tracked remotely and locally
            DrivesCommand::Ls => {
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
            DrivesCommand::Create { path } => {
                let path = path.unwrap_or(current_dir()?);
                let drive_id =
                    name_of(&path).ok_or(ConfigStateError::ExpectedPath(path.clone()))?;
                // Save location association
                global.set_path(&drive_id, &path);
                global.encode(&GlobalConfigId).await?;
                let user_key_id = global.selected_user_key_id()?;
                let id = DriveAndKeyId {
                    drive_id: drive_id.clone(),
                    user_key_id: user_key_id.clone(),
                };

                if let Ok(client) = global.get_client().await {
                    let public_key = SigningKey::decode(&user_key_id).await?.verifying_key();
                    let _remote_id =
                        platform::drives::create(&client, &drive_id, &public_key).await?;
                    let _bfs = SyncBanyanFS::init(client, &id).await?;
                    info!("<< CREATED REMOTE DRIVE >>");
                } else {
                    // Create and encode the Drive and Store
                    LocalBanyanFS::init(&id).await?;
                    info!("<< CREATED LOCAL DRIVE >>");
                }

                Ok(())
            }
            DrivesCommand::Prepare {
                ds,
                follow_links: _,
            } => {
                let mut ld = SyncLoadedDrive::load(&ds.into(), &global).await?;
                //let mut ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                operations::prepare(&mut ld.bfs.drive, &mut ld.bfs.store, &ld.path).await?;
                ld.bfs.encode(&ld.id).await?;
                info!("<< DRIVE DATA STORED SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());
                Ok(())
            }
            DrivesCommand::Delete(ds) => {
                let ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                global.remove_path(&ld.id.drive_id)?;
                Drive::erase(&ld.id).await?;
                LocalBanyanFS::erase(&ld.id).await?;
                global.encode(&GlobalConfigId).await?;

                info!("<< DRIVE DATA DELETED SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());

                Ok(())
            }
            DrivesCommand::Restore(ds) => {
                //let client = global.api_client().await?;
                //let drive = platform::drives::get(&client, drive_id).await?;

                let mut ld = LocalLoadedDrive::load(&ds.into(), &global).await?;
                operations::restore(&mut ld.bfs.drive, &mut ld.bfs.store, &ld.path).await?;
                info!("<< DRIVE DATA RESTORED TO DISK SUCCESSFULLY >>");
                info!("{:?}", ld.bfs.drive.id());
                Ok(())
            }
            DrivesCommand::Sync(ds) => {
                let client = global.get_client().await?;
                let _remote_drives = platform::drives::get_all(&client).await?;
                let _di: DriveId = ds.into();
                //if let Ok(drive_id) = di.get_id().await { }

                //let remote = if let DriveId::DriveId(id) = di { }
                // There is already a local drive here
                //if let Ok(_ld) = LoadedDrive::load(&di, &global).await {}
                todo!()
            }
            DrivesCommand::Access { subcommand } => subcommand.run_internal().await,
        }
    }
}
