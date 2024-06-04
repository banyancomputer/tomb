use super::LocalDataStore;
use banyanfs::stores::{ApiSyncableStore, MemorySyncTracker};

pub type SyncDataStore = ApiSyncableStore<LocalDataStore, MemorySyncTracker>;

/*
#[async_trait(?Send)]
impl OnDisk<String> for SyncDataStore {
    const TYPE: DiskType = DiskType::LocalShare;
    const SUFFIX: &'static str = "drive_sync";
    const EXTENSION: &'static str = "sync";

    async fn encode(&self, identifier: &String) -> Result<(), OnDiskError> {
        let writer = Self::get_writer(identifier).await?;
        let json_data = serde_json::to_string(&(tracked, deleted))?;
        writer
            .compat_write()
            .write_all(json_data.as_bytes())
            .await?;
        Ok(())
    }

    async fn decode(identifier: &String) -> Result<Self, OnDiskError> {
        let reader = Self::get_reader(identifier).await?;
        let mut json_string = String::new();
        let _ = reader.compat().read_to_string(&mut json_string).await?;
        let (tracked, deleted): (Vec<Cid>, Vec<Cid>) = serde_json::from_str(&json_string)?;
        let mut tracker = MemorySyncTracker::default();

        todo!()
    }
}

impl From<DataStoreError> for OnDiskError {
    fn from(value: DataStoreError) -> Self {
        Self::Implementation(format!("{value}"))
    }
}
*/
