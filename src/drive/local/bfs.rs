
use self::local_share::DriveAndKeyId;

use crate::{on_disk::*};

use async_trait::async_trait;

use banyanfs::prelude::*;
use banyanfs::{codec::crypto::SigningKey, utils::crypto_rng};

use std::fs::create_dir_all;


use super::LocalDataStore;

/// Pairs BanyanFS Drives with the ObjectStores which handle their CIDs
pub struct LocalBanyanFS {
    /// BanyanFS Drive
    pub drive: Drive,
    /// Stores CIDs on behalf of the Drive
    pub store: LocalDataStore,
}

impl LocalBanyanFS {
    pub async fn init(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
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
        let store = LocalDataStore::new(store_path)?;

        let lbfs = Self { store, drive };
        lbfs.encode(identifier).await?;
        Ok(lbfs)
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
        // Just save the drive, the data store is already saved deterministically in the location
        OnDisk::encode(&self.drive, identifier).await
    }

    async fn decode(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        // Load the drive using the key
        let drive: Drive = OnDisk::decode(identifier).await?;
        // Create a new
        let store = LocalDataStore::new(Self::path(identifier)?)?;
        Ok(Self { drive, store })
    }
}
