use async_trait::async_trait;
use banyanfs::{
    codec::Cid,
    stores::{DataStore, DataStoreError},
};

pub struct DiskDataStore {}

#[async_trait(?Send)]
impl DataStore for DiskDataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        todo!()
    }

    async fn remove(&mut self, cid: Cid, _recusrive: bool) -> Result<(), DataStoreError> {
        todo!()
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        Ok(())
    }

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        _immediate: bool,
    ) -> Result<(), DataStoreError> {
        todo!()
    }
}
