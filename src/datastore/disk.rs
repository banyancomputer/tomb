use async_trait::async_trait;
use banyanfs::{
    codec::Cid,
    stores::{DataStore, DataStoreError},
};
use object_store::{local::LocalFileSystem, path::Path, ObjectStore};

pub struct DiskDataStore {
    fs: LocalFileSystem,
    drive_name: String,
}

impl DiskDataStore {
    pub fn new_at_path(prefix: &Path) -> Self {
        let fs = LocalFileSystem::new_with_prefix(prefix).unwrap();
        Self {
            fs,
            drive_name: prefix.to_string(),
        }
    }

    fn cid_as_path(&self, cid: &Cid) -> Path {
        Path::parse(&format!(
            "/{}/{}",
            self.drive_name,
            cid.as_base64url_multicodec()
        ))
        .unwrap()
    }
}

#[async_trait(?Send)]
impl DataStore for DiskDataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        match self.fs.head(&self.cid_as_path(&cid)).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { path: _, source: _ }) => Ok(false),
            Err(_) => Err(DataStoreError::LookupFailure),
        }
    }

    async fn remove(&mut self, cid: Cid, _recusrive: bool) -> Result<(), DataStoreError> {
        self.fs
            .delete(&self.cid_as_path(&cid))
            .await
            .map_err(|_| DataStoreError::StoreFailure)
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        let result = self
            .fs
            .get(&self.cid_as_path(&cid))
            .await
            .map_err(|_| DataStoreError::RetrievalFailure)?;

        Ok(result
            .bytes()
            .await
            .map_err(|_| DataStoreError::RetrievalFailure)?
            .to_vec())
    }

    async fn store(
        &mut self,
        cid: Cid,
        data: Vec<u8>,
        _immediate: bool,
    ) -> Result<(), DataStoreError> {
        self.fs
            .put(&self.cid_as_path(&cid), data.into())
            .await
            .map_err(|_| DataStoreError::StoreFailure)
            .map(|_| ())
    }
}
