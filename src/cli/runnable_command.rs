use std::fmt::Display;

use crate::{
    on_disk::{config::GlobalConfig, OnDisk},
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
        if GlobalConfig::decode(&"main".to_string()).await.is_err() {
            GlobalConfig::default()
                .encode(&"main".to_string())
                .await
                .expect("new config");
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
