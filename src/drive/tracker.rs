use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use banyanfs::prelude::*;
use banyanfs::stores::{MemorySyncTracker, SyncTracker};
use serde::{Deserialize, Serialize};

pub struct DiskSyncTracker {
    parent: PathBuf,
    pending_deletion: HashSet<Cid>,
    tracked: HashMap<Cid, u64>,
}

impl DiskSyncTracker {
    fn save(&self) -> Result<(), DataStoreError> {
        let path = self.parent.join("tracker.json");
        let mut writer = std::fs::File::open(&path).map_err(|_| {
            DataStoreError::Implementation(String::from("unable to open local file"))
        })?;
        serde_json::to_writer(&mut writer, self).map_err(|_| {
            DataStoreError::Implementation(String::from("unable to open write to local file"))
        })
    }
}

impl Serialize for DiskSyncTracker {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let pending_deletion = self
            .pending_deletion
            .iter()
            .map(|cid| cid.as_base64url_multicodec())
            .collect::<Vec<String>>();

        let tracked = self
            .tracked
            .iter()
            .map(|(cid, v)| (cid.as_base64url_multicodec(), v.clone()))
            .collect::<Vec<(String, u64)>>();

        (pending_deletion, tracked).serialize(serializer)
    }
}

impl Deserialize for DiskSyncTracker {}

#[async_trait(?Send)]
impl SyncTracker for DiskSyncTracker {
    async fn clear_deleted(&mut self) -> Result<(), DataStoreError> {
        self.pending_deletion.clear();
        self.save()?;
        Ok(())
    }

    async fn delete(&mut self, cid: Cid) -> Result<(), DataStoreError> {
        self.pending_deletion.insert(cid);
        self.save()?;
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
        self.save()?;
        Ok(())
    }
}
