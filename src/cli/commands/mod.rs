use clap::Subcommand;

/// View / Modify Drive Access
//mod access;
/// Login / Logout Account
mod account;
/// Drive access management
//mod drive_access;
/// Drive interaction
mod drives;
/// User Keys
mod keys;
/// View / Change Platform endpoint
mod platform;

/// Export all commands
//pub use access::*;
pub use account::*;
pub use drives::*;
pub use platform::*;
//pub use drives::*;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum BanyanCommand {
    /// Manually configure platform endpoints
    Api {
        /// Subcommand
        #[clap(subcommand)]
        command: platform::PlatformCommand,
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
    /// Key management
    Keys {
        /// Subcommand
        #[clap(subcommand)]
        command: keys::KeysCommand,
    },
}

use super::RunnableCommand;
use crate::{on_disk::config::GlobalConfig, NativeError};
use async_trait::async_trait;
#[async_trait(?Send)]
impl RunnableCommand<NativeError> for BanyanCommand {
    type Payload = GlobalConfig;
    async fn run(self, payload: Self::Payload) -> Result<(), NativeError> {
        match self {
            BanyanCommand::Api { command } => Ok(command.run(()).await?),
            BanyanCommand::Account { command } => Ok(command.run(()).await?),
            BanyanCommand::Drives { command } => command.run(payload).await,
            BanyanCommand::Keys { command } => command.run(()).await,
        }
    }
}
