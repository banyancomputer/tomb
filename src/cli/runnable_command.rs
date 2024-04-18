use std::fmt::Display;

use crate::{
    config::{config_path, GlobalConfig},
    NativeError,
};
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
use tracing::*;

/// Async function for running a command
#[async_trait(?Send)]
pub trait RunnableCommand<ErrorType>: Subcommand
where
    ErrorType: std::error::Error + std::fmt::Debug + Display,
{
    /// The internal running operation
    async fn run_internal(self) -> Result<String, ErrorType>;

    /// Run the internal command, passing a reference to a global configuration which is saved after completion
    async fn run(self) -> Result<(), ErrorType> {
        if !config_path().exists() {
            GlobalConfig::new().await.expect("new config");
        }

        let result = self.run_internal().await;

        // Provide output based on that
        match result {
            Ok(message) => {
                info!("{}", message);
                Ok(())
            }
            Err(error) => {
                error!("{}", format!("{}", error).red());
                Err(error)
            }
        }
    }
}
