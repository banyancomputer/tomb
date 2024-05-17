use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use banyanfs::prelude::*;
use banyanfs::stores::SyncTracker;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

use crate::on_disk::{DiskType, OnDisk, OnDiskError};

#[derive(Clone, Serialize, Deserialize)]
pub struct DiskSyncTracker {
    drive_id: String,
    pending_deletion: HashSet<Cid>,
    tracked: HashMap<Cid, u64>,
}

impl DiskSyncTracker {
    pub fn new(drive_id: &String) -> Self {
        Self {
            drive_id: drive_id.to_string(),
            pending_deletion: HashSet::new(),
            tracked: HashMap::new(),
        }
    }
}

#[async_trait(?Send)]
impl OnDisk<String> for DiskSyncTracker {
    const TYPE: DiskType = DiskType::LocalShare;
    const SUFFIX: &'static str = "drive_sync";
    const EXTENSION: &'static str = "sync";

    async fn encode(&self, identifier: &String) -> Result<(), OnDiskError> {
        let writer = Self::get_writer(identifier).await?;
        let json_data = serde_json::to_string_pretty(&self)?;
        writer
            .compat_write()
            .write_all(json_data.as_bytes())
            .await?;
        Ok(())
    }

    async fn decode(identifier: &String) -> Result<Self, OnDiskError> {
        let reader = Self::get_reader(identifier).await?;
        let mut json_string = String::new();
        reader.compat().read_to_string(&mut json_string).await?;
        Ok(serde_json::from_str(&json_string)?)
    }
}

#[async_trait(?Send)]
impl SyncTracker for DiskSyncTracker {
    async fn clear_deleted(&mut self) -> Result<(), DataStoreError> {
        self.pending_deletion.clear();
        self.encode(&self.drive_id).await?;
        Ok(())
    }

    async fn delete(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.pending_deletion.insert(cid);
        self.encode(&self.drive_id).await?;
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
        self.encode(&self.drive_id).await?;
        Ok(())
    }
}

impl From<OnDiskError> for DataStoreError {
    fn from(value: OnDiskError) -> Self {
        DataStoreError::Implementation(value.to_string())
    }
}
