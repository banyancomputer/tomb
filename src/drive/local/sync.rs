use super::{DiskSyncTracker, LocalDataStore};
use banyanfs::stores::ApiSyncableStore;

pub type SyncDataStore = ApiSyncableStore<LocalDataStore, DiskSyncTracker>;
