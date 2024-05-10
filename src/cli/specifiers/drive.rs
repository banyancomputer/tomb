use clap::Args;
use std::path::PathBuf;
use uuid::Uuid;

/// Unified way of specifying a Bucket
#[derive(Debug, Clone, Args)]
#[group(required = true, multiple = false)]
pub struct DriveSpecifier {
    /// Drive Id
    #[arg(short, long)]
    pub drive_id: Option<Uuid>,
    /// Bucket name
    #[arg(short, long)]
    pub name: Option<String>,
    /// Bucket Root on disk
    #[arg(short, long)]
    pub origin: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum DriveId {
    DriveId(String),
    Name(String),
    Origin(PathBuf),
}

impl From<DriveSpecifier> for DriveId {
    fn from(value: DriveSpecifier) -> Self {
        if let Some(drive_id) = value.drive_id {
            return Self::DriveId(drive_id.to_string());
        }
        if let Some(name) = value.name {
            return Self::Name(name.to_string());
        }
        return Self::Origin(value.origin.expect("failure"));
    }
}
