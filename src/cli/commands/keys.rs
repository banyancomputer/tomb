use crate::{
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk,
    },
    NativeError,
};

use super::RunnableCommand;
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
use url::Url;

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum KeysCommand {
    /// Display the currently selected key
    Display,
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
            KeysCommand::Display => todo!(),
            KeysCommand::Create => todo!(),
            KeysCommand::Select { fingerprint } => todo!(),
        }
    }
}
