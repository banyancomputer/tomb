use crate::{
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk,
    },
    NativeError,
};

use super::RunnableCommand;
use async_trait::async_trait;
use banyanfs::{api::platform::account::*, codec::crypto::SigningKey};
use bytesize::ByteSize;
use clap::Subcommand;
use colored::Colorize;
use tracing::info;

/// Subcommand for Authentication
#[derive(Subcommand, Clone, Debug)]
pub enum AccountCommand {
    /// Add Device API Key using browser session
    Login,
    /// Log out from this device
    Logout,
    /// Get info about Account usage
    Usage,
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for AccountCommand {
    async fn run_internal(self) -> Result<(), NativeError> {
        let global = GlobalConfig::decode(&GlobalConfigId).await?;

        // Process the command
        match self {
            AccountCommand::Login => {
                let key_management_url = format!("{}/account/manage-keys", env!("ENDPOINT"));
                info!("Navigate to {}", key_management_url);

                let user_key_id = global.selected_user_key_id()?;
                let user_key: SigningKey = OnDisk::decode(&user_key_id).await?;
                let public_key = user_key.verifying_key().to_spki().unwrap();

                info!("public_key:\n{}", public_key);

                global.api_client().await?;

                // Respond
                info!(
                    "{}\nuser_id:\t\t{}\ndevice_key_fingerprint:\t{}",
                    "<< DEVICE KEY SUCCESSFULLY ADDED TO ACCOUNT >>".green(),
                    "NO ID",
                    "NO FINGERPRINT",
                    //user_id,
                    //fingerprint
                );
                Ok(())
            }
            AccountCommand::Logout => {
                /*
                client.logout();
                global.save_client(client).await?;
                */
                info!(
                    "{}",
                    "<< SUCCESSFULLY LOGGED OUT OF REMOTE ACCESS >>".green()
                );
                Ok(())
            }
            AccountCommand::Usage => {
                let mut client = global.api_client().await?;
                let mut output = format!("{}", "| ACCOUNT USAGE INFO |".yellow());

                let current_usage_result = current_usage(&mut client).await;
                let usage_limit_result = current_usage_limit(&mut client).await;

                if current_usage_result.is_err() && usage_limit_result.is_err() {
                    return Err(NativeError::Custom(String::from(
                        "Unable to obtain usage stats. Check your authentication!",
                    )));
                }

                if let Ok(usage_current) = current_usage_result {
                    output = format!(
                        "{}\nusage_current:\t{}",
                        output,
                        ByteSize(usage_current.total_usage() as u64)
                    );
                }
                if let Ok(usage_limit) = usage_limit_result {
                    output = format!(
                        "{}\nsoft hot usage limit:\t{}\nhard hot usage limit:\t{}",
                        output,
                        ByteSize(usage_limit.soft_hot_storage_limit() as u64),
                        ByteSize(usage_limit.hard_hot_storage_limit() as u64)
                    );
                }

                info!(output);
                Ok(())
            }
        }
    }
}
