//mod access;
pub mod helpers;
mod operations;

use crate::{
    cli::{
        commands::RunnableCommand,
        specifiers::{DriveId, DriveSpecifier},
        Persistence,
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
use banyanfs::{api::platform, filesystem::Drive};
use operations::DriveOperation;
pub use operations::DriveOperationPayload;

use clap::Subcommand;
use cli_table::{print_stdout, Cell, Table};

use std::{env::current_dir, path::PathBuf};
use tracing::{debug, info, warn};

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
        /// Drive name
        path: Option<PathBuf>,
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
    Delete {
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
            | Delete { name }
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
                    Delete { .. } => DriveOperation::Delete.run(payload).await,
                    Rename { new_name, .. } => {
                        DriveOperation::Rename { new_name }.run(payload).await
                    }
                    _ => panic!(),
                }
            }
            // List all Buckets tracked remotely and locally
            Ls => {
                let api_drives = helpers::api_drives(&global).await;
                debug!("fetched remote drives");

                let mut table_rows = Vec::new();
                let local_drive_names = Drive::entries();

                // Find the drives that exist both locally and remotely
                let mut sync_names = Vec::new();
                for local_name in local_drive_names.iter() {
                    for api in api_drives.iter() {
                        if *local_name == api.name {
                            sync_names.push(local_name.clone());
                            table_rows.push(vec![
                                api.name.clone().cell(),
                                api.id.clone().cell(),
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

                for remote in api_drives.into_iter() {
                    if !sync_names.contains(&remote.name) {
                        table_rows.push(vec![
                            remote.name.clone().cell(),
                            remote.id.clone().cell(),
                            "N/A".cell(),
                            Persistence::RemoteOnly.cell(),
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
            // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
            Create { path } => {
                let path = path.to_owned().unwrap_or(current_dir()?);
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
            } //Access { subcommand } => subcommand.run_internal(payload).await,

              /*
              Operation { name, subcommand } => {
                  let payload = DriveOperationPayload {
                      id: DriveAndKeyId {
                          drive_id: name,
                          user_key_id: global.selected_user_key_id()?,
                      },
                      global: global.clone(),
                  };
                  subcommand.run_internal(payload).await
              }
              */
        }
    }
}
