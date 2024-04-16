use crate::native::{configuration::globalconfig::GlobalConfig, sync::OmniDrive, NativeError};

use super::{
    super::specifiers::{AccessSpecifier, DriveSpecifier},
    RunnableCommand,
};
use async_trait::async_trait;
use banyanfs::api::platform::drive_access;
use clap::Subcommand;
use tracing::info;

/// Subcommand for Drive Keys
#[derive(Subcommand, Clone, Debug)]
pub enum KeyCommand {
    /// Request Access to a Drive if you dont already have it
    RequestAccess(DriveSpecifier),
    /// List all users with Drive access
    Ls(DriveSpecifier),
    /// Revoke a User's ability to access a Drive
    Revoke(AccessSpecifier),
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for KeyCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        let global = GlobalConfig::from_disk().await?;
        let mut client = global.get_client().await?;
        match self {
            KeyCommand::RequestAccess(drive_specifier) => {
                let private_key = global.user_key().await?;
                let public_key = private_key.verifying_key();
                // Compute PEM
                let fingerprint = &public_key.fingerprint().as_hex_id();
                let pem = public_key.to_spki();

                // Get Drive
                let omni = OmniDrive::from_specifier(&drive_specifier).await;
                if let Ok(id) = omni.get_id() {
                    let existing_keys =
                        drive_access::get_all(&mut client, &omni.get_id()?.to_string()).await?;
                    if let Some(existing_key) = existing_keys
                        .iter()
                        .find(|key| key.fingerprint() == fingerprint)
                    {
                        info!("\n{:?}\n", existing_key);
                        Err(NativeError::custom_error(
                            "You've already requested access on this Bucket!",
                        ))
                    } else {
                        /* actually create it now */
                        panic!("unimplemented");
                    }
                } else {
                    Err(NativeError::missing_remote_drive())
                }
            }
            KeyCommand::Ls(drive_specifier) => {
                let omni = OmniDrive::from_specifier(&drive_specifier).await;
                let id = omni.get_id().unwrap();
                drive_access::get_all(&mut client, &omni.get_id()?.to_string())
                    .await
                    .map(|keys| {
                        keys.iter().fold(String::new(), |acc, access| {
                            format!("{}\n\n{:?}", acc, access)
                        })
                    })
                    .map_err(NativeError::api)
            }
            KeyCommand::Revoke(ks) => {
                panic!("unimplemented");
            }
        }
    }
}
