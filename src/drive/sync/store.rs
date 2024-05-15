use crate::{
    drive::{local::LocalDataStore, sync::DiskSyncTracker},
    on_disk::{local_share::DriveAndKeyId, DiskType, OnDisk, OnDiskError},
};

use async_trait::async_trait;
use banyanfs::{api::ApiClient, stores::ApiSyncableStore};

pub type SyncDataStore = ApiSyncableStore<LocalDataStore, DiskSyncTracker>;
