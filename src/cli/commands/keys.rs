use crate::{
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk, OnDiskExt,
    },
    NativeError,
};

use super::RunnableCommand;
use async_trait::async_trait;
use banyanfs::codec::crypto::{Fingerprint, SigningKey, VerifyingKey};
use clap::Subcommand;
use colored::Colorize;
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
                let fingerprints: Vec<String> = SigningKey::decode_all()
                    .await?
                    .into_iter()
                    .map(|key| key.verifying_key().fingerprint())
                    .map(|fingerprint| format!("{fingerprint:?}"))
                    .collect();
                println!("all_fingies: {:?}", fingerprints);

                if fingerprints.is_empty() {
                    Ok(format!("{}", "<< NO KEYS ON DISK, CREATE ONE >>".blue()))
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
            KeysCommand::Create => todo!(),
            KeysCommand::Select { fingerprint } => todo!(),
        }
    }
}
