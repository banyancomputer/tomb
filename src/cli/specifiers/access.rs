use super::DriveSpecifier;
use clap::Args;

/// Unified way of specifying a Key
#[derive(Debug, Clone, Args)]
pub struct AccessSpecifier {
    #[clap(flatten)]
    pub(crate) drive_specifier: DriveSpecifier,
    /// User Key fingerprint
    #[arg(short, long)]
    pub(crate) fingerprint: String,
}
