use crate::{
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk, OnDiskExt,
    },
    utils::prompt_for_bool,
    ConfigStateError, NativeError,
};

use super::RunnableCommand;
use async_trait::async_trait;
use banyanfs::{codec::crypto::SigningKey, utils::crypto_rng};
use clap::Subcommand;
use colored::Colorize;
use tracing::{info, warn};

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum KeysCommand {
    /// List User Keys on disk and show which is selected
    Ls,
    /// Create a new Key
    Create,
    /// Select a key
    Select {
        /// Server address
        #[arg(short, long)]
        fingerprint: String,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for KeysCommand {
    async fn run_internal(self) -> Result<(), NativeError> {
        let mut global = GlobalConfig::decode(&GlobalConfigId).await?;
        match self {
            KeysCommand::Ls => {
                // Collect the public key fingerprints of every private user key
                let fingerprints: Vec<String> = SigningKey::decode_all()
                    .await?
                    .into_iter()
                    .map(|key| key.verifying_key().fingerprint())
                    .map(|fingerprint| format!("{fingerprint:?}"))
                    .collect();

                if fingerprints.is_empty() {
                    warn!("<< NO KEYS ON DISK; CREATE ONE >>");
                } else {
                    info!("<< KEY FINGERPRINTS >>");
                    for fingerprint in fingerprints.into_iter() {
                        info!("{fingerprint}");
                    }
                }
                Ok(())
            }
            KeysCommand::Create => {
                let mut rng = crypto_rng();
                let new_key = SigningKey::generate(&mut rng);
                let fingerprint = format!("{:?}", new_key.fingerprint());
                // Save on disk
                new_key.encode(&fingerprint).await?;
                // Update the config if the user so wishes
                if prompt_for_bool("Select this key for use?") {
                    global.select_user_key_id(fingerprint);
                    global.encode(&GlobalConfigId).await?;
                    info!("<< PREFERENCE SAVED >>");
                }
                info!("<< KEY CREATED >>");
                Ok(())
            }
            KeysCommand::Select { fingerprint } => {
                // If we can successfully load the key
                if SigningKey::decode(&fingerprint).await.is_ok() {
                    // Update the config
                    global.select_user_key_id(fingerprint);
                    global.encode(&GlobalConfigId).await?;
                    info!("<< PREFERENCE SAVED >>");
                    Ok(())
                } else {
                    Err(ConfigStateError::MissingKey(fingerprint).into())
                }
            }
        }
    }
}
