use std::fmt::Display;

use async_trait::async_trait;

/// Async function for running a command
#[async_trait(?Send)]
pub trait RunnableCommand<ErrorType>
where
    ErrorType: std::error::Error + std::fmt::Debug + Display,
{
    type Payload;
    /// The internal running operation
    async fn run(self, payload: Self::Payload) -> Result<(), ErrorType>;
}
