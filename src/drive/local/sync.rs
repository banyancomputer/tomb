use super::{CborSyncTracker, LocalDataStore};
use banyanfs::stores::{ApiSyncableStore, MemorySyncTracker};

pub type SyncDataStore = ApiSyncableStore<LocalDataStore, CborSyncTracker>;
