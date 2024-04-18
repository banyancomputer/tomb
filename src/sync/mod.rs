//mod local;
use colored::Colorize;
//pub use local::LocalDrive;
use std::fmt::Display;

/// Sync State
#[derive(Debug, Clone, PartialEq)]
pub enum SyncState {
    /// Initial / Default state
    Unknown,
    /// There is no remote correlate
    Unpublished,
    /// There is no local correlate
    Unlocalized,
    /// Local bucket is behind the remote
    Behind,
    /// Local and remote are congruent
    MetadataSynced,
    /// Local and remote are congruent
    AllSynced,
    /// Local bucket is ahead of the remote
    Ahead,
}

impl Display for SyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = match self {
            SyncState::Unknown => "Unknown".red(),
            SyncState::Unpublished => "Drive does not exist remotely".red(),
            SyncState::Unlocalized => "Drive does not exist locally".red(),
            SyncState::Behind => "Drive is behind remote".red(),
            SyncState::MetadataSynced => "Metadata Synced; File System not reconstructed".blue(),
            SyncState::AllSynced => "Drive is in sync with remote".green(),
            SyncState::Ahead => "Drive is ahead of remote".red(),
        };

        f.write_fmt(format_args!("{}", description))
    }
}
