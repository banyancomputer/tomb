use std::fmt::Display;

use async_trait::async_trait;
use clap::Subcommand;

/// Async function for running a command
#[async_trait(?Send)]
pub trait RunnableCommand<ErrorType>: Subcommand
where
    ErrorType: std::error::Error + std::fmt::Debug + Display,
{
    type Payload;
    /// The internal running operation
    async fn run_internal(self, payload: Self::Payload) -> Result<(), ErrorType>;
}
