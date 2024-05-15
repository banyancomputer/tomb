use crate::{
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk,
    },
    utils::prompt_for_uuid,
    NativeError,
};

use super::RunnableCommand;
use async_trait::async_trait;
use banyanfs::{api::platform::account::*, codec::crypto::SigningKey};
use bytesize::ByteSize;
use clap::Subcommand;

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
        let mut global = GlobalConfig::decode(&GlobalConfigId).await?;

        // Process the command
        match self {
            AccountCommand::Login => {
                let key_management_url = format!("{}/account/manage-keys", env!("ENDPOINT"));
                info!("Navigate to {}", key_management_url);
                let user_key_id = global.selected_user_key_id()?;
                let user_key: SigningKey = OnDisk::decode(&user_key_id).await?;
                let public_key = user_key.verifying_key().to_spki().unwrap();
                info!("public_key:\n{}", public_key);
                let account_id = prompt_for_uuid("Enter your account id:");
                global.set_account_id(&account_id)?;
                let _ = global.api_client().await?;
                global.encode(&GlobalConfigId).await?;
                info!("<< SUCCESSFULLY LOGGED IN >>");
                Ok(())
            }
            AccountCommand::Logout => {
                global.remove_account_id();
                global.encode(&GlobalConfigId).await?;
                info!("<< SUCCESSFULLY LOGGED OUT OF REMOTE ACCESS >>");
                Ok(())
            }
            AccountCommand::Usage => {
                let mut client = global.api_client().await?;
                info!("| ACCOUNT USAGE INFO |");
                let current_usage_result = current_usage(&mut client).await;
                let usage_limit_result = current_usage_limit(&mut client).await;

                if current_usage_result.is_err() && usage_limit_result.is_err() {
                    return Err(NativeError::Custom(String::from(
                        "Unable to obtain usage stats. Check your authentication!",
                    )));
                }

                if let Ok(usage_current) = current_usage_result {
                    info!("hot usage:\t\t\t{}", ByteSize(usage_current.hot_usage()));
                    info!(
                        "archival usage:\t\t{}",
                        ByteSize(usage_current.archival_usage())
                    );
                }
                if let Ok(usage_limit) = usage_limit_result {
                    info!(
                        "soft hot usage limit:\t{}",
                        ByteSize(usage_limit.soft_hot_storage_limit() as u64),
                    );
                    info!(
                        "hard hot usage limit:\t{}",
                        ByteSize(usage_limit.hard_hot_storage_limit() as u64)
                    );
                }

                Ok(())
            }
        }
    }
}
