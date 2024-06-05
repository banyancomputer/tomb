use super::RunnableCommand;
use crate::NativeError;
use async_trait::async_trait;
use clap::Subcommand;

use cli_table::{print_stdout, Cell, Table};
use tracing::info;
use url::Url;

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum PlatformCommand {
    /// Display the current platform endpoint
    Display,
    /// Set the endpoint to a new value
    Set {
        /// Server address
        #[arg(short, long)]
        address: String,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for PlatformCommand {
    type Payload = ();

    async fn run(self, _payload: ()) -> Result<(), NativeError> {
        match self {
            PlatformCommand::Display => {
                let table = vec![vec![env!("ENDPOINT").cell()]]
                    .table()
                    .title(vec!["Remote Address".cell()]);
                print_stdout(table)?;
                Ok(())
            }
            PlatformCommand::Set { address } => {
                let _ = Url::parse(&address).map_err(|err| NativeError::Custom(err.to_string()));
                std::env::set_var("ENDPOINT", address);
                info!("<< ENDPOINT UPDATED SUCCESSFULLY >>");
                Ok(())
            }
        }
    }
}
