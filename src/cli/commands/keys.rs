use std::collections::HashSet;

use crate::{
    cli::display::{Persistence, TableEntry},
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk, OnDiskExt,
    },
    utils::{prompt_for_bool, prompt_for_key_name, prompt_for_string},
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
    /// Delete a key
    Rm {
        /// Key name
        name: String,
    },
    /// Select a key for use
    Select {
        /// Key name
        name: String,
    },
    /// Display the currently selected key
    Selected,
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for KeysCommand {
    type Payload = ();
    async fn run_internal(self, _payload: ()) -> Result<(), NativeError> {
        let mut global = GlobalConfig::decode(&GlobalConfigId).await?;
        use KeysCommand::*;
        match self {
            Ls => {
                let remote_keys: Vec<ApiUserKey> = if let Ok(client) = global.get_client().await {
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

                // Collect the public key fingerprints of every private user key
                let local_named_keys: Vec<(String, SigningKey)> = SigningKey::decode_all().await?;
                if local_named_keys.is_empty() {
                    warn!("<< NO KEYS ON DISK; CREATE ONE >>");
                    return Ok(());
                }

                let mut sync_names = HashSet::new();
                let mut table_rows = Vec::new();

                for (local_name, local_private_key) in local_named_keys.iter() {
                    let local_public_key = local_private_key.verifying_key();
                    let local_fingerprint = api_fingerprint_key(&local_public_key);

                    for remote_key in remote_keys.iter() {
                        // Sync key found
                        if remote_key.fingerprint() == local_fingerprint {
                            // Ensure name congruence
                            // If the names are different for some reason
                            let remote_name = remote_key.name();
                            let key_name = if remote_name != local_name {
                                warn!(
                                    "Remote key with name `{}` is named `{}` locally.",
                                    remote_name, local_name
                                );
                                if prompt_for_bool("Keep local or remote name?", 'l', 'r') {
                                    info!("Renaming remote key.");
                                    let client = global.get_client().await?;
                                    platform::account::rename_user_key(
                                        &client,
                                        local_name,
                                        remote_key.id(),
                                    )
                                    .await?;
                                    local_name
                                } else {
                                    info!("Renaming local key.");
                                    // Write by new name, erase by old
                                    local_private_key.encode(remote_name).await?;
                                    SigningKey::erase(local_name).await?;

                                    // Handle config
                                    if let Ok(selected_user_key_id) = global.selected_user_key_id()
                                    {
                                        if selected_user_key_id == *local_name {
                                            global.select_user_key_id(remote_name.to_string());
                                            global.encode(&GlobalConfigId).await?;
                                        }
                                    }
                                    remote_name
                                }
                            } else {
                                local_name
                            };

                            //
                            sync_names.insert(local_name);
                            sync_names.insert(remote_name);
                            table_rows.push(vec![
                                key_name.cell(),
                                remote_key.user_id().cell(),
                                remote_key.fingerprint().cell(),
                                remote_key.api_access().cell(),
                                remote_key.public_key().cell(),
                                Persistence::Sync.cell(),
                            ])
                        }
                    }
                }

                for (local_name, private_key) in local_named_keys.iter() {
                    if !sync_names.contains(local_name) {
                        let public_key = private_key.verifying_key();
                        let fingerprint = api_fingerprint_key(&public_key);
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
                            Persistence::LocalOnly.cell(),
                        ]);
                    }
                }

                for remote_key in remote_keys.iter() {
                    if !sync_names.contains(remote_key.name()) {
                        table_rows.push(vec![
                            remote_key.name().cell(),
                            remote_key.user_id().cell(),
                            remote_key.fingerprint().cell(),
                            remote_key.api_access().cell(),
                            remote_key.public_key().cell(),
                            Persistence::RemoteOnly.cell(),
                        ])
                    }
                }

                let table = table_rows.table().title(vec![
                    "Name".cell(),
                    "User ID".cell(),
                    "Fingerprint".cell(),
                    "API".cell(),
                    "Public Key".cell(),
                    "Persistence".cell(),
                ]);

                print_stdout(table)?;

                Ok(())
            }
            Create => {
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
            Rm { name } => {
                // If we can successfully load the key
                if SigningKey::decode(&name).await.is_ok() {
                    warn!("This is private key material. This operation will erase it from your local machine. Use with caution.");
                    if name == prompt_for_string("Re-enter the name of your key to confirm") {
                        SigningKey::erase(&name).await?;
                        // Make sure we also delesect the key if it was in use
                        if let Ok(selected_user_key_id) = global.selected_user_key_id() {
                            if selected_user_key_id == name {
                                global.deselect_user_key_id();
                                global.encode(&GlobalConfigId).await?;
                            }
                        }
                        info!("Erased key.");
                    } else {
                        warn!("Key names don't match.");
                    }
                    Ok(())
                } else {
                    Err(ConfigStateError::MissingKey(name).into())
                }
            }
            Select { name } => {
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
            Selected => {
                let selected_user_key_id = global.selected_user_key_id()?;
                let private_key = SigningKey::decode(&selected_user_key_id).await?;
                let private_key_path = SigningKey::path(&selected_user_key_id)?;
                let public_key = private_key.verifying_key();
                let fingerprint = api_fingerprint_key(&public_key);
                let public_key = public_key.to_spki().unwrap();

                let table = vec![vec![
                    selected_user_key_id.cell(),
                    fingerprint.cell(),
                    public_key.cell(),
                    private_key_path.display().cell(),
                ]]
                .table()
                .title(vec![
                    "Name".cell(),
                    "Fingerprint".cell(),
                    "Public Key".cell(),
                    "Private Key Path".cell(),
                ]);
                print_stdout(table)?;
                Ok(())
            }
        }
    }
}
