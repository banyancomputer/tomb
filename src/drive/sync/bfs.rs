use super::store::SyncDataStore;
use banyanfs::filesystem::Drive;

/// Pairs BanyanFS Drives with the ObjectStores which handle their CIDs
pub struct SyncBanyanFS {
    /// BanyanFS Drive
    pub drive: Drive,
    /// Stores CIDs on behalf of the Drive
    pub store: SyncDataStore,
}
