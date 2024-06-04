/// Commands to run
pub mod commands;
/// Command Trait
mod runnable_command;
/// Ways of specifying resources
pub mod specifiers;
/// Debug level
pub mod verbosity;
/// Export
pub use runnable_command::RunnableCommand;

/// CLI Args
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Command passed
    #[command(subcommand)]
    pub command: commands::BanyanCommand,
    /// Verbosity level.
    #[arg(short, long, help = "verbosity level", default_value = "normal")]
    pub verbose: verbosity::MyVerbosity,
}

#[derive(Debug)]
pub enum Persistence {
    LocalOnly,
    RemoteOnly,
    Sync,
}

impl std::fmt::Display for Persistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Persistence::LocalOnly => f.write_str("Local Only"),
            Persistence::RemoteOnly => f.write_str("Remote Only"),
            Persistence::Sync => f.write_str("Sync"),
        }
    }
}
