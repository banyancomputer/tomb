use super::DiskDataStore;
use banyanfs::stores::{ApiSyncableStore, MemorySyncTracker};

pub type DiskApiSyncStore = ApiSyncableStore<DiskDataStore, MemorySyncTracker>;
