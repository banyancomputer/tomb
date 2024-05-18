use crate::{
    cli::display::{TableAble, TableEntry},
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk, OnDiskExt,
    },
    utils::{prompt_for_bool, prompt_for_key_name},
    ConfigStateError, NativeError,
};

use super::RunnableCommand;
use async_trait::async_trait;
use banyanfs::{
    api::{
        api_fingerprint_key,
        platform::{self, ApiUserKey},
    },
    codec::crypto::SigningKey,
    utils::crypto_rng,
};
use clap::Subcommand;

use cli_table::{print_stdout, Cell, Table};
use tracing::{info, warn};

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum KeysCommand {
    /// List User Keys on disk and show which is selected
    Ls,
    /// Create a new Key
    Create,
    /// Select a key for use
    Select {
        /// Key name
        #[arg(short, long)]
        name: String,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for KeysCommand {
    async fn run_internal(self) -> Result<(), NativeError> {
        let mut global = GlobalConfig::decode(&GlobalConfigId).await?;
        match self {
            KeysCommand::Ls => {
                let mut remote_keys: Vec<ApiUserKey> = if let Ok(client) = global.get_client().await
                {
                    info!("Fetching remote keys, too.");
                    platform::account::user_key_access(&client)
                        .await
                        .map_err(|err| {
                            warn!("Error requesting user keys from remote: {err:?}");
                        })
                        .unwrap_or(vec![])
                        .into_iter()
                        .map(|uka| uka.key)
                        .collect()
                } else {
                    vec![]
                };

                let mut table_rows = Vec::new();

                // Collect the public key fingerprints of every private user key
                let local_named_keys: Vec<(String, SigningKey)> = SigningKey::decode_all().await?;
                if local_named_keys.is_empty() {
                    warn!("<< NO KEYS ON DISK; CREATE ONE >>");
                    return Ok(());
                }

                for (local_name, private_key) in local_named_keys.iter() {
                    let public_key = private_key.verifying_key();
                    let fingerprint = api_fingerprint_key(&public_key);
                    if let Some(remote) = remote_keys
                        .iter()
                        .find(|api_key| api_key.fingerprint() == fingerprint)
                    {
                        // If the names are different for some reason
                        let remote_name = remote.name();
                        if remote_name != local_name {
                            warn!(
                                "Remote key with name `{}` is named `{}` locally.",
                                remote_name, local_name
                            );
                            if prompt_for_bool("Rename local or remote?", 'l', 'r') {
                                info!("Renaming key locally.");
                                // Write by new name, erase by old
                                private_key.encode(remote_name).await?;
                                SigningKey::erase(local_name).await?;

                                // Handle config
                                if let Ok(selected_user_key_id) = global.selected_user_key_id() {
                                    if selected_user_key_id == *local_name {
                                        global.select_user_key_id(remote_name.to_string());
                                        global.encode(&GlobalConfigId).await?;
                                    }
                                }
                            } else {
                                info!("Renaming key remotely.");
                                let client = global.get_client().await?;
                                platform::account::rename_user_key(
                                    &client,
                                    local_name,
                                    remote.id(),
                                )
                                .await?;
                                remote_keys = platform::account::user_key_access(&client)
                                    .await
                                    .map_err(|err| {
                                        warn!("Error requesting user keys from remote: {err:?}");
                                    })
                                    .unwrap_or(vec![])
                                    .into_iter()
                                    .map(|uka| uka.key)
                                    .collect();
                            }
                        }
                    }
                    // If the local key isn't known by the server
                    else {
                        // List it manually
                        table_rows.push(vec![
                            local_name.cell(),
                            "N/A".cell(),
                            fingerprint.cell(),
                            false.cell(),
                            public_key
                                .to_spki()
                                .map_err(|_| NativeError::Custom("Spki".to_string()))?
                                .cell(),
                            false.cell(),
                        ])
                    }
                }

                table_rows.extend(remote_keys.rows());
                let table = table_rows.table().title(ApiUserKey::title());
                print_stdout(table)?;

                Ok(())
            }
            KeysCommand::Create => {
                let mut rng = crypto_rng();
                let new_key = SigningKey::generate(&mut rng);
                let new_key_id = prompt_for_key_name("Name this Key:")?;
                // Save on disk
                new_key.encode(&new_key_id).await?;
                // Update the config if the user so wishes
                if prompt_for_bool("Select this key for use?", 'y', 'n') {
                    global.select_user_key_id(new_key_id);
                    global.encode(&GlobalConfigId).await?;
                }
                info!("<< KEY CREATED >>");
                Ok(())
            }
            KeysCommand::Select { name } => {
                // If we can successfully load the key
                if SigningKey::decode(&name).await.is_ok() {
                    // Update the config
                    global.select_user_key_id(name);
                    global.encode(&GlobalConfigId).await?;
                    Ok(())
                } else {
                    Err(ConfigStateError::MissingKey(name).into())
                }
            }
        }
    }
}
