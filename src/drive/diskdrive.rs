use self::local_share::DriveAndKeyId;
use super::datastore::OnDiskDataStore;
use crate::on_disk::*;
use async_trait::async_trait;
use banyanfs::prelude::*;
use banyanfs::{codec::crypto::SigningKey, utils::crypto_rng};
use object_store::local::LocalFileSystem;
use std::fs::create_dir_all;
use std::path::PathBuf;

pub struct DiskDriveAndStore {
    pub store: OnDiskDataStore,
    pub drive: Drive,
}

impl DiskDriveAndStore {
    async fn init(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        let mut rng = crypto_rng();
        let current_key = SigningKey::decode(&identifier.user_key_id).await?;
        let drive = Drive::initialize_private(&mut rng, current_key.into())
            .map_err(|err| OnDiskError::Implementation(err.to_string()))?;
        let store_path = Self::path(identifier)?;
        if !store_path.exists() {
            create_dir_all(&store_path)?;
        }
        let store = OnDiskDataStore {
            lfs: LocalFileSystem::new_with_prefix(store_path.display().to_string())
                .map_err(|err| OnDiskError::Implementation(err.to_string()))?,
        };

        let ddas = Self { store, drive };
        ddas.encode(identifier).await?;
        Ok(ddas)
    }
}

#[async_trait(?Send)]
impl OnDisk<DriveAndKeyId> for DiskDriveAndStore {
    const TYPE: DiskType = DiskType::LocalShare;
    const SUFFIX: &'static str = "drive_data";
    const EXTENSION: &'static str = "";

    async fn encode(&self, identifier: &DriveAndKeyId) -> Result<(), OnDiskError> {
        OnDisk::encode(&self.drive, identifier).await
    }
    async fn decode(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        let drive: Drive = OnDisk::decode(identifier).await?;
        let store = OnDiskDataStore {
            lfs: LocalFileSystem::new_with_prefix(Self::path(identifier)?.display().to_string())
                .map_err(|err| OnDiskError::Implementation(err.to_string()))?,
        };
        Ok(Self { drive, store })
    }
}
