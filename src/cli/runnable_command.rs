use std::fmt::Display;

use crate::on_disk::{
    config::{GlobalConfig, GlobalConfigId},
    DiskType, OnDisk,
};
use async_trait::async_trait;
use clap::Subcommand;

use tracing::*;

/// Async function for running a command
#[async_trait(?Send)]
pub trait RunnableCommand<ErrorType>: Subcommand
where
    ErrorType: std::error::Error + std::fmt::Debug + Display,
{
    /// The internal running operation
    async fn run_internal(self) -> Result<(), ErrorType>;

    /// Run the internal command, passing a reference to a global configuration which is saved after completion
    async fn run(self) -> Result<(), ErrorType> {
        if GlobalConfig::decode(&GlobalConfigId).await.is_err() {
            DiskType::Config.init().expect("creating configs");
            DiskType::LocalShare.init().expect("creating configs");

            GlobalConfig::default()
                .encode(&GlobalConfigId)
                .await
                .expect("new config");
        }

        let result = self.run_internal().await;
        if let Err(err) = &result {
            error!("{err}");
        }
        result
    }
}
