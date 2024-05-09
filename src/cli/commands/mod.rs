use clap::Subcommand;

/// View / Modify Drive Access
//mod access;
/// Login / Logout Account
mod account;
/// View / Change API endpoint
mod api;
/// Drive interaction
mod drives;

/// Export all commands
//pub use access::*;
pub use account::*;
pub use api::*;
//pub use drives::*;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum BanyanCommand {
    /// Manually configure remote endpoints
    Api {
        /// Subcommand
        #[clap(subcommand)]
        command: api::ApiCommand,
    },
    /// Account Login and Details
    Account {
        /// Subcommand
        #[clap(subcommand)]
        command: account::AccountCommand,
    },
    /// Drive management
    Drives {
        /// Subcommand
        #[clap(subcommand)]
        command: drives::DrivesCommand,
    },
}

use super::RunnableCommand;
use crate::NativeError;
use async_trait::async_trait;
#[async_trait(?Send)]
impl RunnableCommand<NativeError> for BanyanCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        match self {
            BanyanCommand::Api { command } => Ok(command.run_internal().await?),
            BanyanCommand::Account { command } => Ok(command.run_internal().await?),
            BanyanCommand::Drives { command } => command.run_internal().await,
        }
    }
}
