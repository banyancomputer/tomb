/// Commands to run
pub mod commands;
/// CLI Output Impls
pub mod display;
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
