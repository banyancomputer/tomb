
use clap::Args;
use std::path::PathBuf;
use uuid::Uuid;

use crate::{
    on_disk::{config::GlobalConfig},
    utils::name_of,
    ConfigStateError, NativeError,
};

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

impl DriveId {
    pub async fn get_id(&self, _global: &GlobalConfig) -> Result<String, NativeError> {
        match self {
            // This will require either cached values in the Global config, OR it will require just
            // asking the API directly (preferable)
            DriveId::DriveId(drive_id) => {
                Err(ConfigStateError::MissingDrive(drive_id.to_string()).into())
            }
            DriveId::Name(name) => Ok(name.to_string()),
            DriveId::Origin(origin) => name_of(origin)
                .ok_or(ConfigStateError::MissingDrive(format!("{}", origin.display())).into()),
        }
    }
}
