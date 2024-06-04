use std::{
    collections::{HashMap, HashSet},
    io::Write,
};

use async_trait::async_trait;
use banyanfs::prelude::*;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

use crate::on_disk::{DiskType, OnDisk, OnDiskError};
/// A minimal implementation of a memory backed sync tracker. This implementation
/// is currently used by our WASM implementation for tracking which blocks are
/// stored where, but also represents the minimal amount of work that others
/// would need to implement to create an alternate block tracking system.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct CborSyncTracker {
    pending_deletion: HashSet<Cid>,
    tracked: HashMap<Cid, u64>,
}

#[async_trait(?Send)]
impl SyncTracker for CborSyncTracker {
    async fn clear_deleted(&mut self) -> Result<(), DataStoreError> {
        self.pending_deletion.clear();
        Ok(())
    }

    async fn delete(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.pending_deletion.insert(cid);
        Ok(())
    }

    async fn deleted_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        Ok(self.pending_deletion.iter().cloned().collect())
    }

    async fn track(&mut self, cid: Cid, size: u64) -> Result<(), DataStoreError> {
        self.tracked.entry(cid).or_insert(size);
        Ok(())
    }

    async fn tracked_cids(&self) -> Result<Vec<Cid>, DataStoreError> {
        Ok(self.tracked.keys().cloned().collect())
    }

    async fn tracked_size(&self) -> Result<u64, DataStoreError> {
        Ok(self.tracked.values().sum())
    }

    async fn untrack(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.tracked.remove(&cid);
        Ok(())
    }
}

/// ~/.local/share/banyan/drive_sync
/// Contains .sync files representing sync tracking
#[async_trait(?Send)]
impl OnDisk<String> for CborSyncTracker {
    const TYPE: DiskType = DiskType::LocalShare;
    const SUFFIX: &'static str = "drive_sync";
    const EXTENSION: &'static str = "sync";

    async fn encode(&self, identifier: &String) -> Result<(), OnDiskError> {
        let writer = Self::get_std_writer(identifier).await?;
        tracing::warn!("about to make vec!");
        ciborium::into_writer(&self, &writer)
            .map_err(|err| OnDiskError::Implementation("cbor".into()))?;
        Ok(())
    }

    async fn decode(identifier: &String) -> Result<Self, OnDiskError> {
        let reader = Self::get_std_reader(identifier).await?;
        let tracker: CborSyncTracker = ciborium::from_reader(&reader)
            .map_err(|err| OnDiskError::Implementation("cbor".into()))?;
        Ok(tracker)
    }
}
