mod access;
pub mod helpers;
mod operations;

use crate::{
    cli::{
        commands::{drives::access::DriveAccessPayload, RunnableCommand},
        Persistence,
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
use banyanfs::{api::platform, filesystem::Drive};
use operations::DriveOperation;
pub use operations::DriveOperationPayload;

use clap::Subcommand;
use cli_table::{print_stdout, Cell, Table};

use std::path::PathBuf;
use tracing::{debug, error, info, warn};

use self::{access::DriveAccessCommand, helpers::platform_drive_with_name};

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DrivesCommand {
    /// Information about a specific drive
    Info {
        /// Drive name
        name: String,
    },
    /// List all Drives
    Ls,
    /// Initialize a new Drive
    Create {
        /// Unencrypted source location
        path: PathBuf,
    },
    /// Prepare a Drive for Pushing by encrypting new data
    Prepare {
        /// Drive name
        name: String,
    },
    /// Reconstruct a Drive filesystem locally
    Restore {
        /// Drive name
        name: String,
    },
    /// Delete a Drive
    Rm {
        /// Drive name
        name: String,
    },
    /// Change the name of a Drive
    Rename {
        /// Drive name
        #[arg(short, long)]
        name: String,
        /// New Drive name
        #[arg(short, long)]
        new_name: String,
    },
    /// Drive Key management
    Access {
        name: String,
        /// Subcommand
        #[clap(subcommand)]
        subcommand: DriveAccessCommand,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DrivesCommand {
    type Payload = GlobalConfig;

    async fn run(self, mut global: GlobalConfig) -> Result<(), NativeError> {
        use DrivesCommand::*;
        match &self {
            Info { name }
            | Prepare { name }
            | Restore { name }
            | Rm { name }
            | Rename { name, .. } => {
                let payload = DriveOperationPayload {
                    id: DriveAndKeyId {
                        drive_id: name.to_string(),
                        user_key_id: global.selected_user_key_id()?,
                    },
                    global,
                };

                match self {
                    Info { .. } => DriveOperation::Info.run(payload).await,
                    Prepare { .. } => DriveOperation::Prepare.run(payload).await,
                    Restore { .. } => DriveOperation::Restore.run(payload).await,
                    Rm { .. } => DriveOperation::Rm.run(payload).await,
                    Rename { new_name, .. } => {
                        DriveOperation::Rename { new_name }.run(payload).await
                    }
                    _ => panic!(),
                }
            }
            // List all Buckets tracked on platform and locally
            Ls => {
                let platform_drives = helpers::platform_drives(&global).await;
                debug!("fetched platform drives");

                let mut table_rows = Vec::new();
                let local_drive_names = Drive::entries();

                // Find the drives that exist both locally and on platform
                let mut sync_names = Vec::new();
                for local_name in local_drive_names.iter() {
                    for platform_drive in platform_drives.iter() {
                        if *local_name == platform_drive.name {
                            sync_names.push(local_name.clone());
                            table_rows.push(vec![
                                platform_drive.name.clone().cell(),
                                platform_drive.id.clone().cell(),
                                global.get_path(local_name)?.display().cell(),
                                Persistence::Sync.cell(),
                            ])
                        }
                    }
                }

                debug!("found sync drives");

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

                debug!("found local drives");

                for platform in platform_drives.into_iter() {
                    if !sync_names.contains(&platform.name) {
                        table_rows.push(vec![
                            platform.name.clone().cell(),
                            platform.id.clone().cell(),
                            "N/A".cell(),
                            Persistence::PlatformOnly.cell(),
                        ])
                    }
                }

                debug!("found api drives");

                if table_rows.is_empty() {
                    warn!("No known drives. Make or sync!");
                } else {
                    let table = table_rows.table().title(vec![
                        "Name".cell(),
                        "ID".cell(),
                        "Path".cell(),
                        "Persistence".cell(),
                    ]);
                    print_stdout(table)?;
                }

                Ok(())
            }
            // Create a new Bucket. This attempts to create the Bucket both locally and on platform, but settles for a simple local creation if remote permissions fail
            Create { path } => {
                // Determine the "drive id" (name)
                let drive_id = name_of(path).ok_or(ConfigStateError::ExpectedPath(path.clone()))?;

                if Drive::entries().contains(&drive_id)
                    || platform_drive_with_name(&global, &drive_id).await.is_some()
                {
                    error!("There is already a local or remote drive by that name.");
                    return Ok(());
                }

                // Save location association
                global.set_path(&drive_id, path);
                global.encode(&GlobalConfigId).await?;
                let user_key_id = global.selected_user_key_id()?;
                let id = DriveAndKeyId::new(&drive_id, &user_key_id);

                // Create and encode the Drive and Store
                LocalBanyanFS::init(&id).await?;
                info!("<< CREATED LOCAL DRIVE >>");

                if let Ok(client) = global.get_client().await {
                    let _platform_id = platform::drives::create(&client, &drive_id).await?;
                    info!("<< CREATED PLATFORM DRIVE >>");
                    DriveOperationPayload { id, global }.sync().await?;
                }
                Ok(())
            }
            Access { name, subcommand } => {
                let payload = DriveAccessPayload {
                    id: DriveAndKeyId {
                        drive_id: name.to_string(),
                        user_key_id: global.selected_user_key_id()?,
                    },
                    global,
                };
                subcommand.clone().run(payload).await
            }
        }
    }
}
