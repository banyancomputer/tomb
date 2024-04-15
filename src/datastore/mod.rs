mod disk;
mod error;
mod tracker;

use banyanfs::stores::ApiSyncableStore;
use disk::DiskDataStore;

pub type DataStorage = ApiSyncableStore<DiskDataStore, DiskSyncTracker>;
