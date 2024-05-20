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
use banyanfs::{
    api::platform::{self, account::*},
    codec::crypto::SigningKey,
};
use bytesize::ByteSize;
use clap::Subcommand;

use cli_table::{print_stdout, Cell, Table};
use tracing::info;

/// Subcommand for Authentication
#[derive(Subcommand, Clone, Debug)]
pub enum AccountCommand {
    /// Display current platform account status
    Info,
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
        use AccountCommand::*;
        match self {
            Info => {
                let mut row = vec![];
                if let Ok(account_id) = global.get_account_id() {
                    row.push(account_id.to_string().cell());
                    info!("")
                } else {
                    row.push("None".cell());
                }

                if let Ok(selected_user_key_id) = global.selected_user_key_id() {
                    row.push(selected_user_key_id.cell());
                } else {
                    row.push("None".cell());
                }

                let table = vec![row]
                    .table()
                    .title(vec!["Account ID".cell(), "User Key".cell()]);
                print_stdout(table)?;

                Ok(())
            }
            Login => {
                let key_management_url = format!("{}/account/manage-keys", env!("ENDPOINT"));
                info!("Navigate to {}", key_management_url);
                let user_key_id = global.selected_user_key_id()?;
                let user_key: SigningKey = OnDisk::decode(&user_key_id).await?;
                let public_key = user_key.verifying_key().to_spki().unwrap();
                info!("public_key:");
                println!("{}", public_key);
                let account_id = prompt_for_uuid("Enter your account id:");
                global.set_account_id(&account_id)?;
                let _ = global.get_client().await?;
                global.encode(&GlobalConfigId).await?;
                info!("<< SUCCESSFULLY LOGGED IN >>");
                Ok(())
            }
            Logout => {
                global.remove_account_id();
                global.encode(&GlobalConfigId).await?;
                info!("<< SUCCESSFULLY LOGGED OUT OF REMOTE ACCESS >>");
                Ok(())
            }
            Usage => {
                let client = global.get_client().await?;
                let current_usage_result = current_usage(&client).await;
                let usage_limit_result = current_usage_limit(&client).await;
                if current_usage_result.is_err() && usage_limit_result.is_err() {
                    return Err(NativeError::Custom(String::from(
                        "Unable to obtain usage stats. Check your authentication!",
                    )));
                }
                if let Ok(usage_current) = current_usage_result {
                    let table = vec![
                        vec!["Hot".cell(), ByteSize(usage_current.hot_usage()).cell()],
                        vec![
                            "Archival".cell(),
                            ByteSize(usage_current.archival_usage()).cell(),
                        ],
                    ]
                    .table()
                    .title(vec!["".cell(), "Current Usage".cell()]);
                    print_stdout(table)?;
                }
                if let Ok(usage_limit) = usage_limit_result {
                    let table = vec![
                        vec![
                            "Soft Hot".cell(),
                            ByteSize(usage_limit.soft_hot_storage_limit() as u64).cell(),
                        ],
                        vec![
                            "Hard Hot".cell(),
                            ByteSize(usage_limit.hard_hot_storage_limit() as u64).cell(),
                        ],
                    ]
                    .table()
                    .title(vec!["".cell(), "Usage Limits".cell()]);
                    print_stdout(table)?;
                }

                Ok(())
            }
        }
    }
}
