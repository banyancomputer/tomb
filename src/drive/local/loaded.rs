use std::path::PathBuf;

use crate::{
    cli::commands::*,
    on_disk::{local_share::DriveAndKeyId, OnDisk},
    NativeError,
};

use super::LocalBanyanFS;

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
        let bfs = LocalBanyanFS::decode(&payload.id).await?;
        Ok(Self {
            path,
            id: payload.id.clone(),
            bfs,
        })
    }

    /*
    pub async fn load(di: &DriveId, global: &GlobalConfig) -> Result<Self, NativeError> {
        let drive_id = di.get_id().await?;
        let path = global.get_path(&drive_id)?;
        let user_key_id = global.selected_user_key_id()?;
        let id = DriveAndKeyId {
            drive_id,
            user_key_id,
        };
        let bfs = LocalBanyanFS::decode(&id).await?;
        Ok(Self { path, id, bfs })
    }
    */
}
