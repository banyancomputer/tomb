use self::config::{GlobalConfig, GlobalConfigId};
use self::local_share::DriveAndKeyId;

use crate::{on_disk::*, NativeError};

use async_trait::async_trait;

use banyanfs::api::{platform, VecStream};
use banyanfs::prelude::*;
use banyanfs::{codec::crypto::SigningKey, utils::crypto_rng};
use tracing::{info, warn};

use std::fs::create_dir_all;

use crate::drive::local::{CborSyncTracker, LocalDataStore, SyncDataStore};

/// Pairs BanyanFS Drives with the ObjectStores which handle their CIDs
pub struct LocalBanyanFS {
    /// BanyanFS Drive
    pub drive: Drive,
    /// Stores CIDs on behalf of the Drive
    pub store: LocalDataStore,
    /// Sync Tracker
    pub tracker: CborSyncTracker,
}

impl LocalBanyanFS {
    pub async fn init_from_drive(
        identifier: &DriveAndKeyId,
        drive: Drive,
    ) -> Result<Self, OnDiskError> {
        // Determine where we'll put our cid bins
        let store_path = Self::path(identifier)?;
        // Create dir if needed
        if !store_path.exists() {
            create_dir_all(&store_path)?;
        }
        let store = LocalDataStore::new(store_path)?;
        let tracker = CborSyncTracker::default();
        tracker.encode(&identifier.drive_id).await?;

        let bfs = Self {
            store,
            drive,
            tracker,
        };
        bfs.encode(identifier).await?;
        Ok(bfs)
    }

    pub async fn init(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        let mut rng = crypto_rng();
        // Decode the specified UserKey
        let user_key = SigningKey::decode(&identifier.user_key_id).await?;
        // Initialize a new private Drive with this key
        let drive = Drive::initialize_private(&mut rng, user_key.into())
            .map_err(|err| OnDiskError::Implementation(err.to_string()))?;
        // Init from drive
        Self::init_from_drive(identifier, drive).await
    }

    // TODO REMOVE
    pub async fn go_online(&self) -> Result<SyncDataStore, NativeError> {
        let global = GlobalConfig::decode(&GlobalConfigId).await?;
        let client = global.get_client().await?;
        Ok(SyncDataStore::new(
            client,
            self.store.clone(),
            self.tracker.clone(),
        ))
    }
}

/// ~/.local/share/banyan/drive_blocks
/// Contains one folder per Drive, which in turn
/// contain {cid}.bin files managed by the Drive
#[async_trait(?Send)]
impl OnDisk<DriveAndKeyId> for LocalBanyanFS {
    const TYPE: DiskType = DiskType::LocalShare;
    const SUFFIX: &'static str = "drive_blocks";
    // this is a dir
    const EXTENSION: &'static str = "";

    async fn encode(&self, identifier: &DriveAndKeyId) -> Result<(), OnDiskError> {
        // The data store is already "saved" deterministically in the location and doesn't need
        // explicit encoding here
        warn!("encoding tracker");
        self.tracker.encode(&identifier.drive_id).await?;
        warn!("encoded tracker");
        OnDisk::encode(&self.drive, identifier).await
    }

    async fn decode(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        // Load the tracker
        let tracker = CborSyncTracker::decode(&identifier.drive_id).await?;
        // Load the drive using the key
        let drive: Drive = OnDisk::decode(identifier).await?;
        // Create a new
        let store = LocalDataStore::new(Self::path(identifier)?)?;
        Ok(Self {
            drive,
            store,
            tracker,
        })
    }
}
