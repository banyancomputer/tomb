use crate::{
    cli::{
        commands::RunnableCommand,
        specifiers::{DriveId, DriveSpecifier},
    },
    drive::*,
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        local_share::DriveAndKeyId,
        OnDisk, OnDiskError,
    },
    utils::name_of,
    ConfigStateError, NativeError,
};
use async_trait::async_trait;
use banyanfs::{codec::crypto::SigningKey, filesystem::Drive};
use bytesize::ByteSize;
use clap::Subcommand;
use colored::Colorize;
use std::{env::current_dir, path::PathBuf};
use tracing::info;

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DrivesCommand {
    /// List all Drives
    //Ls,
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
    /// Delete a Drive
    Delete(DriveSpecifier),
    /*
    /// Reconstruct a Drive filesystem locally
    Restore {
        /// Drive in question
        #[clap(flatten)]
        drive_specifier: DriveSpecifier,
    },
    /// Sync Drive data to or from remote
    Sync(DriveSpecifier),
    /// Drive info
    Info(DriveSpecifier),
    /// Drive data usage
    Usage(DriveSpecifier),
    /// Get information on Drive Metadata
    Metadata {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: MetadataCommand,
    },
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
    async fn run_internal(self) -> Result<String, NativeError> {
        let mut global = GlobalConfig::decode(&GlobalConfigId).await?;

        match self {
            /*
            // List all Buckets tracked remotely and locally
            DrivesCommand::Ls => {
                let omnis = OmniBucket::ls().await?;
                if !omnis.is_empty() {
                    Ok(omnis
                        .iter()
                        .fold(String::new(), |acc, bucket| format!("{acc}\n{bucket}")))
                } else {
                    Ok("No known Drives locally or remotely.".to_string())
                }
            }
            */
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
                    drive_id,
                    user_key_id,
                };
                // Create and encode the Drive and Store
                let ddas = DiskDriveAndStore::init(&id).await?;
                Ok(format!(
                    "{}\n{:?}",
                    "<< NEW DRIVE CREATED >>".green(),
                    ddas.drive.id()
                ))
            }
            DrivesCommand::Prepare { ds, follow_links } => {
                let drive_id = Into::<DriveId>::into(ds).get_id().await?;
                let drive_origin = global.get_origin(&drive_id)?;
                println!("drive_id: {drive_id:?}");
                println!("origin: {drive_origin:?}");

                let user_key_id = global.selected_user_key_id()?;
                println!("ukid: {user_key_id:?}");
                let id = DriveAndKeyId {
                    drive_id,
                    user_key_id,
                };
                let mut ddas = DiskDriveAndStore::decode(&id).await?;
                ddas.prepare(&drive_origin).await?;
                ddas.encode(&id).await?;

                Ok(format!(
                    "{}\n{:?}",
                    "<< DRIVE DATA STORED SUCCESSFULLY >>".green(),
                    ddas.drive.id()
                ))
            }
            DrivesCommand::Delete(ds) => {
                let drive_id = Into::<DriveId>::into(ds).get_id().await?;
                global.remove_origin(&drive_id)?;
                let user_key_id = global.selected_user_key_id()?;
                let id = DriveAndKeyId {
                    drive_id,
                    user_key_id,
                };
                let ddas = DiskDriveAndStore::decode(&id).await?;

                Drive::erase(&id).await?;
                DiskDriveAndStore::erase(&id).await?;

                global.encode(&GlobalConfigId).await?;

                Ok(format!(
                    "{}\n{:?}",
                    "<< DRIVE DATA DELETED SUCCESSFULLY >>".green(),
                    ddas.drive.id()
                ))
            } /*
                  DrivesCommand::Restore { drive_specifier } => {
                      restore::pipeline(OmniBucket::from_specifier(&drive_specifier).await).await
                  }
                  DrivesCommand::Sync(drive_specifier) => {
                      OmniBucket::from_specifier(&drive_specifier)
                          .await
                          .sync_bucket()
                          .await
                  }
                  DrivesCommand::Delete(drive_specifier) => {
                      let omni = OmniBucket::from_specifier(&drive_specifier).await;
                      let local_deletion = prompt_for_bool("Do you want to delete this Bucket locally?");
                      let remote_deletion =
                          prompt_for_bool("Do you want to delete this Bucket remotely?");
                      omni.delete(local_deletion, remote_deletion).await
                  }
                  DrivesCommand::Info(drive_specifier) => {
                      let omni = OmniBucket::from_specifier(&drive_specifier).await;
                      Ok(format!("{omni}"))
                  }
                  DrivesCommand::Usage(drive_specifier) => {
                      let mut client = GlobalConfig::from_disk().await?.get_client().await?;
                      let remote = OmniBucket::from_specifier(&drive_specifier)
                          .await
                          .get_remote()?;
                      remote
                          .usage(&mut client)
                          .await
                          .map(|v| {
                              format!(
                                  "{}drive_id:\t\t{}\nusage:\t\t{}",
                                  "| USAGE INFO |".blue(),
                                  remote.id,
                                  ByteSize(v)
                              )
                          })
                          .map_err(NativeError::api)
                  }
                  DrivesCommand::Metadata { subcommand } => subcommand.run_internal().await,
                  DrivesCommand::Keys { subcommand } => subcommand.run_internal().await,
              */
        }
    }
}
