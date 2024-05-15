use std::path::PathBuf;

use crate::{
    cli::specifiers::DriveId,
    on_disk::{config::GlobalConfig, local_share::DriveAndKeyId, OnDisk},
    NativeError,
};

use super::LocalBanyanFS;

pub struct LocalLoadedDrive {
    pub origin: PathBuf,
    pub id: DriveAndKeyId,
    pub bfs: LocalBanyanFS,
}

impl LocalLoadedDrive {
    pub async fn load(di: &DriveId, global: &GlobalConfig) -> Result<Self, NativeError> {
        let drive_id = di.get_id().await?;
        let origin = global.get_origin(&drive_id)?;
        let user_key_id = global.selected_user_key_id()?;
        let id = DriveAndKeyId {
            drive_id,
            user_key_id,
        };
        let bfs = LocalBanyanFS::decode(&id).await?;
        Ok(Self { origin, id, bfs })
    }
}
