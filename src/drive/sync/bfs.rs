use self::config::{GlobalConfig, GlobalConfigId};
use self::local_share::DriveAndKeyId;
use crate::drive::local::LocalDataStore;
use crate::drive::sync::{DiskSyncTracker, SyncDataStore};
use crate::{on_disk::*, NativeError};
use async_recursion::async_recursion;
use async_trait::async_trait;
use banyanfs::codec::filesystem::NodeKind;
use banyanfs::prelude::*;
use banyanfs::{codec::crypto::SigningKey, utils::crypto_rng};
use tracing::info;

use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

/// Pairs BanyanFS Drives with the ObjectStores which handle their CIDs
pub struct SyncBanyanFS {
    /// BanyanFS Drive
    pub drive: Drive,
    /// Stores CIDs on behalf of the Drive
    pub store: SyncDataStore,
}

impl SyncBanyanFS {
    pub async fn init(client: ApiClient, identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        let mut rng = crypto_rng();
        // Decode the specified UserKey
        let user_key = SigningKey::decode(&identifier.user_key_id).await?;
        // Initialize a new private Drive with this key
        let drive = Drive::initialize_private(&mut rng, user_key.into())
            .map_err(|err| OnDiskError::Implementation(err.to_string()))?;

        // Determine where we'll put our cid bins
        let store_path = Self::path(identifier)?;
        // Create dir if needed
        if !store_path.exists() {
            create_dir_all(&store_path)?;
        }
        let dst = DiskSyncTracker::decode(&identifier.drive_id)
            .await
            .unwrap_or(DiskSyncTracker::new(&identifier.drive_id));
        dst.encode(&identifier.drive_id).await?;

        let store = SyncDataStore::new(client, LocalDataStore::new(store_path)?, dst);

        let lbfs = Self { store, drive };
        lbfs.encode(identifier).await?;
        Ok(lbfs)
    }
}

/// ~/.local/share/banyan/drive_blocks
/// Contains one folder per Drive, which in turn
/// contain {cid}.bin files managed by the Drive
#[async_trait(?Send)]
impl OnDisk<DriveAndKeyId> for SyncBanyanFS {
    const TYPE: DiskType = DiskType::LocalShare;
    const SUFFIX: &'static str = "drive_blocks";
    // this is a dir
    const EXTENSION: &'static str = "";

    async fn encode(&self, identifier: &DriveAndKeyId) -> Result<(), OnDiskError> {
        // Just save the drive, the data store is already saved deterministically in the location
        OnDisk::encode(&self.drive, identifier).await
    }

    async fn decode(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        let global = GlobalConfig::decode(&GlobalConfigId).await?;
        let client = global
            .api_client()
            .await
            .map_err(|err| OnDiskError::Implementation("client".to_string()))?;
        // Load the drive using the key
        let drive: Drive = OnDisk::decode(identifier).await?;
        // Create a new
        let dst = DiskSyncTracker::decode(&identifier.drive_id)
            .await
            .unwrap_or(DiskSyncTracker::new(&identifier.drive_id));
        dst.encode(&identifier.drive_id).await?;
        let store = SyncDataStore::new(client, LocalDataStore::new(Self::path(identifier)?)?, dst);
        Ok(Self { drive, store })
    }
}
