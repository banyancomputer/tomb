use crate::{
    drive::{local::LocalDataStore, sync::DiskSyncTracker},
};


use banyanfs::{stores::ApiSyncableStore};

pub type SyncDataStore = ApiSyncableStore<LocalDataStore, DiskSyncTracker>;
