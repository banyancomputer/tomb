use clap::{command, Parser};

use super::{commands::BanyanCommand, verbosity::MyVerbosity};

/// Arguments to tomb
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Command passed
    #[command(subcommand)]
    pub command: BanyanCommand,
    /// Verbosity level.
    #[arg(short, long, help = "verbosity level", default_value = "normal")]
    pub verbose: MyVerbosity,
}
