//! This crate contains all modules in our project.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]

use {
    clap::Parser,
    cli::{Args, RunnableCommand},
    tracing::Level,
    tracing_subscriber::{
        fmt::format, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
    },
};

/// CLI Parsing
pub mod cli;

/// Drive structs
pub mod drive;

/// Local share data
pub mod on_disk;

/// Utils
pub mod utils;

/// Error
mod error;
pub use error::*;

#[tokio::main]
async fn main() {
    let cli = Args::parse();
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stderr());
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::DEBUG.into())
        .from_env_lossy();
    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .event_format(format().compact())
        .without_time()
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_writer(non_blocking_writer)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(stderr_layer).init();

    // Determine the command being executed run appropriate subcommand
    let _ = cli.command.run().await;
}
