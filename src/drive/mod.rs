mod store;
//mod tracker;

use banyanfs::stores::{ApiSyncableStore, MemorySyncTracker};
use store::DiskDataStore;
//use tracker::DiskSyncTracker;

pub type LocalSyncStore = ApiSyncableStore<DiskDataStore, MemorySyncTracker>;
