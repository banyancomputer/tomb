use std::fmt::Display;

use crate::on_disk::{config::GlobalConfig, DiskData, DiskDataError};

use super::RunnableCommand;
use async_trait::async_trait;
use banyanfs::api::{platform::account::*, ApiError};
use bytesize::ByteSize;
use clap::Subcommand;
use colored::Colorize;

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
impl RunnableCommand<AccountCommandError> for AccountCommand {
    async fn run_internal(self) -> Result<String, AccountCommandError> {
        let mut global = GlobalConfig::decode(&String::from("main")).await?;
        let mut client = global.get_client().await?;

        // Process the command
        match self {
            AccountCommand::Login => {
                // there is not currently a way to do this!

                // Respond
                Ok(format!(
                    "{}\nuser_id:\t\t{}\ndevice_key_fingerprint:\t{}",
                    "<< DEVICE KEY SUCCESSFULLY ADDED TO ACCOUNT >>".green(),
                    "NO ID",
                    "NO FINGERPRINT",
                    //user_id,
                    //fingerprint
                ))
            }
            AccountCommand::Logout => {
                /*
                client.logout();
                global.save_client(client).await?;
                */
                Ok(format!(
                    "{}",
                    "<< SUCCESSFULLY LOGGED OUT OF REMOTE ACCESS >>".green()
                ))
            }
            AccountCommand::Usage => {
                let mut output = format!("{}", "| ACCOUNT USAGE INFO |".yellow());

                let current_usage_result = current_usage(&mut client).await;
                let usage_limit_result = current_usage_limit(&mut client).await;

                if current_usage_result.is_err() && usage_limit_result.is_err() {
                    return Err(AccountCommandError::Custom(String::from(
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

                Ok(output)
            }
        }
    }
}

#[derive(Debug)]
pub enum AccountCommandError {
    Api(ApiError),
    Config(DiskDataError),
    Custom(String),
}

impl Display for AccountCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountCommandError::Api(err) => f.write_str(&err.to_string()),
            AccountCommandError::Config(err) => f.write_str(&err.to_string()),
            AccountCommandError::Custom(err) => f.write_str(err),
        }
    }
}

impl std::error::Error for AccountCommandError {}

impl From<DiskDataError> for AccountCommandError {
    fn from(value: DiskDataError) -> Self {
        Self::Config(value)
    }
}

impl From<ApiError> for AccountCommandError {
    fn from(value: ApiError) -> Self {
        Self::Api(value)
    }
}
