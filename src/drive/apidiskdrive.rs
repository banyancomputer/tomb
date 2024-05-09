use super::OnDiskDataStore;
use banyanfs::stores::{ApiSyncableStore, MemorySyncTracker};
pub type DiskApiSyncStore = ApiSyncableStore<OnDiskDataStore, MemorySyncTracker>;
