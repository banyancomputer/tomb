use clap::Subcommand;

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DriveAccessCommand {
    /// List drive actors
    Actors,
    /// Grant access to a known key
    Grant,
    /// Revoke access from a known key
    Revoke,
}
