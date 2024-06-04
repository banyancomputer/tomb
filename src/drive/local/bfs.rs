use self::config::{GlobalConfig, GlobalConfigId};
use self::local_share::DriveAndKeyId;

use crate::{on_disk::*, NativeError};

use async_trait::async_trait;

use banyanfs::api::{platform, VecStream};
use banyanfs::prelude::*;
use banyanfs::{codec::crypto::SigningKey, utils::crypto_rng};
use tracing::{info, warn};

use std::fs::create_dir_all;

use crate::drive::local::{DiskSyncTracker, LocalDataStore, SyncDataStore};

/// Pairs BanyanFS Drives with the ObjectStores which handle their CIDs
pub struct LocalBanyanFS {
    /// BanyanFS Drive
    pub drive: Drive,
    /// Stores CIDs on behalf of the Drive
    pub store: LocalDataStore,
    /// Sync Tracker
    pub tracker: DiskSyncTracker,
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
        let tracker = DiskSyncTracker::new(&identifier.drive_id);
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

    pub async fn go_online(&self) -> Result<SyncDataStore, NativeError> {
        let global = GlobalConfig::decode(&GlobalConfigId).await?;
        let client = global.get_client().await?;
        Ok(SyncDataStore::new(
            client,
            self.store.clone(),
            self.tracker.clone(),
        ))
    }

    pub async fn sync(&self, bucket_id: &str) -> Result<(), NativeError> {
        let global = GlobalConfig::decode(&GlobalConfigId).await?;
        let client = global.get_client().await?;
        let mut store = self.go_online().await?;

        let mut rng = crypto_rng();
        // For Metadata push
        let content_options = ContentOptions::metadata();
        let mut encoded_drive = Vec::new();
        self.drive
            .encode(&mut rng, content_options, &mut encoded_drive)
            .await?;
        let expected_data_size = store.unsynced_data_size().await?;
        let root_cid = self.drive.root_cid().await?;

        let verifying_keys: Vec<VerifyingKey> = self
            .drive
            .verifying_keys()
            .await
            .into_iter()
            .filter_map(|(key, mask)| {
                if !mask.is_historical() {
                    Some(key)
                } else {
                    None
                }
            })
            .collect();

        let deleted_block_cids = store.deleted_cids().await?;
        let drive_stream = VecStream::new(encoded_drive).pinned();

        let push_response = platform::metadata::push_stream(
            &client,
            bucket_id,
            expected_data_size,
            root_cid,
            //self.last_saved_metadata.as_ref().map(|m| m.id()).clone(),
            None,
            drive_stream,
            verifying_keys,
            deleted_block_cids,
        )
        .await?;
        let new_metadata_id = push_response.id();

        if let Some(host) = push_response.storage_host() {
            if let Err(err) = store.set_sync_host(host.clone()).await {
                // In practice this should never happen, the trait defines an error type for
                // flexibility in the future but no implementations currently produce an error.
                warn!("failed to set sync host: {err}");
            }
            if let Some(grant) = push_response.storage_authorization() {
                client.record_storage_grant(host, grant).await;
            }
        }

        let _new_metadata = platform::metadata::get(&client, bucket_id, &new_metadata_id).await?;
        if let Err(err) = store.sync(&new_metadata_id).await {
            warn!("failed to sync data store to remotes, data remains cached locally but unsynced and can be retried: {err}");
            // note(sstelfox): this could be recoverable with future syncs, but we
            // should probably still fail here...
            return Err(NativeError::Custom(
                "failed to sync data store to remotes".into(),
            ));
        }

        info!("drive synced");

        Ok(())
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
        info!("getting tracker");
        // Load the tracker
        let tracker = DiskSyncTracker::decode(&identifier.drive_id).await?;
        info!("getting drive");
        // Load the drive using the key
        let drive: Drive = OnDisk::decode(identifier).await?;
        info!("getting store");
        // Create a new
        let store = LocalDataStore::new(Self::path(identifier)?)?;
        Ok(Self {
            drive,
            store,
            tracker,
        })
    }
}
