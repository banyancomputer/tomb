use crate::drive::local::{DiskSyncTracker, LocalDataStore};

use banyanfs::stores::ApiSyncableStore;

pub type SyncDataStore = ApiSyncableStore<LocalDataStore, DiskSyncTracker>;
