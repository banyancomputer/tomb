use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DriveConfig {
    /// DiskDrive location
    pub root: PathBuf,
    /// Userland filesystem
    pub origin: PathBuf,
}
