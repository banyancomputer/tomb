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

use cli_table::{print_stdout, Cell, Table};
use tracing::info;
use url::Url;

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum ApiCommand {
    /// Display the current remote endpoint
    Display,
    /// Set the endpoint to a new value
    Set {
        /// Server address
        #[arg(short, long)]
        address: String,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for ApiCommand {
    async fn run_internal(self) -> Result<(), NativeError> {
        let _global = GlobalConfig::decode(&GlobalConfigId).await?;
        match self {
            ApiCommand::Display => {
                let table = vec![vec![env!("ENDPOINT").cell()]]
                    .table()
                    .title(vec!["Remote Address".cell()]);
                print_stdout(table)?;
                Ok(())
            }
            ApiCommand::Set { address } => {
                let _ = Url::parse(&address).map_err(|err| NativeError::Custom(err.to_string()));
                std::env::set_var("ENDPOINT", address);
                info!("<< ENDPOINT UPDATED SUCCESSFULLY >>");
                Ok(())
            }
        }
    }
}
