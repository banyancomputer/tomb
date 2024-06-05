/// Commands to run
pub mod commands;
/// Convenience functions for working in user space
mod helpers;
/// Command Trait
mod runnable_command;
/// Export
pub use runnable_command::RunnableCommand;

/// CLI Args
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Command passed
    #[command(subcommand)]
    pub command: commands::BanyanCommand,
}

#[derive(Debug)]
pub enum Persistence {
    LocalOnly,
    PlatformOnly,
    Sync,
}

impl std::fmt::Display for Persistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Persistence::LocalOnly => f.write_str("Local Only"),
            Persistence::PlatformOnly => f.write_str("Platform Only"),
            Persistence::Sync => f.write_str("Sync"),
        }
    }
}
