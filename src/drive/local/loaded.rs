use std::path::PathBuf;

use tracing::info;

use crate::{
    cli::commands::*,
    on_disk::{local_share::DriveAndKeyId, OnDisk},
    NativeError,
};

use super::LocalBanyanFS;

// TODO remove
pub struct LocalLoadedDrive {
    /// Location of the Drive in user space
    pub path: PathBuf,
    /// Unique identifier for the loaded Drive
    pub id: DriveAndKeyId,
    /// Local BFS
    pub bfs: LocalBanyanFS,
}

impl LocalLoadedDrive {
    pub async fn load(payload: &DriveOperationPayload) -> Result<Self, NativeError> {
        let path = payload.global.get_path(&payload.id.drive_id)?;
        info!("got path: {}", path.display());
        let bfs = LocalBanyanFS::decode(&payload.id).await?;
        Ok(Self {
            path,
            id: payload.id.clone(),
            bfs,
        })
    }
}
