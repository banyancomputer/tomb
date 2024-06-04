//mod access;
mod operations;

use crate::{
    cli::{
        commands::RunnableCommand,
        display::Persistence,
        specifiers::{DriveId, DriveSpecifier},
    },
    drive::local::*,
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
use operations::DriveOperationCommand;
pub use operations::DriveOperationPayload;

use clap::Subcommand;
use cli_table::{print_stdout, Cell, Table};

use std::{env::current_dir, path::PathBuf};
use tracing::{debug, error, info, warn};

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
    /// Operate on a known drive
    Operation {
        #[clap(flatten)]
        drive_specifier: DriveSpecifier,

        #[clap(subcommand)]
        subcommand: DriveOperationCommand,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DrivesCommand {
    type Payload = GlobalConfig;

    async fn run_internal(self, mut global: GlobalConfig) -> Result<(), NativeError> {
        use DrivesCommand::*;
        match self {
            // List all Buckets tracked remotely and locally
            Ls => {
                let remote_drives = match global.get_client().await {
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
                };

                debug!("fetched remote drives");

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

                debug!("found remote drives");

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
                    //let _remote_id = platform::drives::create(&client, &drive_id).await?;
                    //info!("<< CREATED REMOTE DRIVE >>");
                }

                // Create and encode the Drive and Store
                LocalBanyanFS::init(&id).await?;
                info!("<< CREATED LOCAL DRIVE >>");

                Ok(())
            }
            Operation {
                drive_specifier,
                subcommand,
            } => {
                let drive_id = DriveId::from(drive_specifier).get_id().await?;
                let payload = DriveOperationPayload {
                    id: DriveAndKeyId {
                        drive_id,
                        user_key_id: global.selected_user_key_id()?,
                    },
                    global: global.clone(),
                };
                subcommand.run_internal(payload).await
            }
        }
    }
}
