use std::fmt::Display;

use crate::types::config::globalconfig::GlobalConfig;
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
use tomb_common::banyan_api::client::Client;

/// Async function for running a command
#[async_trait(?Send)]
pub trait RunnableCommand<ErrorType>: Subcommand
where
    ErrorType: Into<Box<dyn std::error::Error>> + std::fmt::Debug + Display,
{
    /// The internal running operation
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, ErrorType>;

    /// Run the internal command, passing a reference to a global configuration which is saved after completion
    async fn run(self) -> Result<(), ErrorType> {
        // Grab global config
        let mut global = GlobalConfig::from_disk()
            .await
            .expect("unable to load global config");
        let mut client = global.get_client().await.expect("unable to load client");
        let result = self.run_internal(&mut global, &mut client).await;
        global
            .save_client(client)
            .await
            .expect("unable to save client to config");
        global.to_disk().expect("Unable to save global config");

        // Provide output based on that
        match result {
            Ok(message) => {
                println!("{}", message);
                Ok(())
            }
            Err(error) => {
                println!("{}", format!("{}", error).red());
                Err(error)
            }
        }
    }
}
