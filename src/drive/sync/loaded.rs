use std::path::PathBuf;

use crate::{
    cli::specifiers::DriveId,
    on_disk::{config::GlobalConfig, local_share::DriveAndKeyId, OnDisk},
    NativeError,
};

use super::SyncBanyanFS;

pub struct SyncLoadedDrive {
    pub path: PathBuf,
    pub id: DriveAndKeyId,
    pub bfs: SyncBanyanFS,
}

impl SyncLoadedDrive {
    pub async fn load(di: &DriveId, global: &GlobalConfig) -> Result<Self, NativeError> {
        let drive_id = di.get_id().await?;
        let path = global.get_path(&drive_id)?;
        let user_key_id = global.selected_user_key_id()?;
        let id = DriveAndKeyId {
            drive_id,
            user_key_id,
        };
        let bfs = SyncBanyanFS::decode(&id).await?;
        Ok(Self { path, id, bfs })
    }
}
