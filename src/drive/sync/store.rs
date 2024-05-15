use crate::{
    drive::{local::LocalDataStore, sync::DiskSyncTracker},
    on_disk::{local_share::DriveAndKeyId, DiskType, OnDisk, OnDiskError},
};

use async_trait::async_trait;
use banyanfs::{api::ApiClient, stores::ApiSyncableStore};

pub type SyncDataStore = ApiSyncableStore<LocalDataStore, DiskSyncTracker>;

/*
#[async_trait(?Send)]
impl OnDisk<DriveAndKeyId> for DiskApiSyncStore {
    const TYPE: DiskType = DiskType::LocalShare;
    const SUFFIX: &'static str = "drive_blocks";
    // this is a dir
    const EXTENSION: &'static str = "";

    async fn encode(&self, identifier: &DriveAndKeyId) -> Result<(), OnDiskError> {
        // Just save the drive, the data store is already saved deterministically in the location
        OnDisk::encode(&self.drive, identifier).await
    }

    async fn decode(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        //let c = ApiClient::new
        // Load the drive using the key
        let drive: Drive = OnDisk::decode(identifier).await?;
        // Create a new
        let store = OnDiskDataStore::new(Self::path(identifier)?)?;
        Ok(Self { drive, store })
    }
}
*/
