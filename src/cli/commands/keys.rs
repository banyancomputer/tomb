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
use banyanfs::{
    codec::crypto::{Fingerprint, SigningKey, VerifyingKey},
    utils::crypto_rng,
};
use clap::Subcommand;
use colored::Colorize;
use tracing::info;
use url::Url;

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
    async fn run_internal(self) -> Result<String, NativeError> {
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
                    Ok(format!("{}", "<< NO KEYS ON DISK; CREATE ONE >>".blue()))
                } else {
                    Ok(format!(
                        "{}\n{}",
                        "<< KEY FINGERPRINTS >>".green(),
                        fingerprints
                            .into_iter()
                            .fold(String::new(), |acc, f| format!("{acc}\n{f}"))
                    ))
                }
            }
            KeysCommand::Create => {
                let mut rng = crypto_rng();
                let new_key = SigningKey::generate(&mut rng);
                let fingerprint = format!("{:?}", new_key.fingerprint());
                // Save on disk
                new_key.encode(&fingerprint).await?;
                // Update the config if the user so wishes
                if prompt_for_bool("Select this key for use?") {
                    global.selected_user_key_id = Some(fingerprint);
                    global.encode(&GlobalConfigId).await?;
                    info!("{}", "<< PREFERENCE SAVED >>".green());
                }
                Ok(format!("{}", "<< KEY CREATED >>".green()))
            }
            KeysCommand::Select { fingerprint } => {
                // If we can successfully load the key
                if SigningKey::decode(&fingerprint).await.is_ok() {
                    // Update the config
                    global.selected_user_key_id = Some(fingerprint);
                    global.encode(&GlobalConfigId).await?;
                    Ok(format!("{}", "<< PREFERENCE SAVED >>".green()))
                } else {
                    Err(ConfigStateError::MissingKey(fingerprint).into())
                }
            }
        }
    }
}
