use clap::Subcommand;

/// View / Modify Drive Access
mod access;
/// Login / Logout Account
mod account;
/// View / Change API endpoint
mod api;
///
mod drives;
/// View Drive Metadata? do we even want this
//mod metadata;
mod runnable_command;

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

use crate::native::NativeError;
use async_trait::async_trait;
use runnable_command::RunnableCommand;
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
