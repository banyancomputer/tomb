use self::local_share::DriveAndKeyId;
use super::datastore::DiskDataStore;
use crate::on_disk::*;
use async_trait::async_trait;
use banyanfs::prelude::*;
use banyanfs::{codec::crypto::SigningKey, utils::crypto_rng};
use object_store::local::LocalFileSystem;
use std::fs::create_dir_all;

pub struct DiskDriveAndStore {
    store: DiskDataStore,
    drive: Drive,
}

impl DiskDriveAndStore {
    async fn init(identifier: &DriveAndKeyId) -> Result<Self, DiskDataError> {
        let mut rng = crypto_rng();
        let current_key = SigningKey::decode(&identifier.user_key_id).await?;
        let drive = Drive::initialize_private(&mut rng, current_key.into())
            .map_err(|err| DiskDataError::Implementation(err.to_string()))?;
        let store_path = Self::path(identifier)?;
        if !store_path.exists() {
            create_dir_all(&store_path)?;
        }
        let store = DiskDataStore {
            lfs: LocalFileSystem::new_with_prefix(store_path.display().to_string())
                .map_err(|err| DiskDataError::Implementation(err.to_string()))?,
        };

        let ddas = Self { store, drive };
        ddas.encode(identifier).await?;
        Ok(ddas)
    }
}

#[async_trait(?Send)]
impl DiskData<DriveAndKeyId> for DiskDriveAndStore {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: &'static str = "drive_data";
    const EXTENSION: &'static str = "";

    async fn encode(&self, identifier: &DriveAndKeyId) -> Result<(), DiskDataError> {
        DiskData::encode(&self.drive, identifier).await
    }
    async fn decode(identifier: &DriveAndKeyId) -> Result<Self, DiskDataError> {
        let drive: Drive = DiskData::decode(identifier).await?;
        let store = DiskDataStore {
            lfs: LocalFileSystem::new_with_prefix(Self::path(identifier)?.display().to_string())
                .map_err(|err| DiskDataError::Implementation(err.to_string()))?,
        };
        Ok(Self { drive, store })
    }
}
