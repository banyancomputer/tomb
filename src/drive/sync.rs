use super::{CborSyncTracker, LocalDataStore};
use banyanfs::stores::ApiSyncableStore;

pub type SyncDataStore = ApiSyncableStore<LocalDataStore, CborSyncTracker>;
