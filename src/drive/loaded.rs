use std::path::PathBuf;

use crate::{
    cli::specifiers::{DriveId, DriveSpecifier},
    on_disk::{config::GlobalConfig, local_share::DriveAndKeyId, OnDisk},
    NativeError,
};

use super::DiskDriveAndStore;

pub struct LoadedDrive {
    pub origin: PathBuf,
    pub id: DriveAndKeyId,
    pub ddas: DiskDriveAndStore,
}

impl LoadedDrive {
    pub async fn load(ds: &DriveSpecifier, global: &GlobalConfig) -> Result<Self, NativeError> {
        let drive_id = Into::<DriveId>::into(ds.clone()).get_id().await?;
        let origin = global.get_origin(&drive_id)?;
        let user_key_id = global.selected_user_key_id()?;
        let id = DriveAndKeyId {
            drive_id,
            user_key_id,
        };
        let ddas = DiskDriveAndStore::decode(&id).await?;
        Ok(Self { origin, id, ddas })
    }
}
