use std::path::PathBuf;

use async_trait::async_trait;
use banyanfs::{
    codec::Cid,
    stores::{DataStore, DataStoreError},
};
use object_store::{local::LocalFileSystem, path::Path, ObjectStore};

use crate::on_disk::OnDiskError;

pub struct LocalDataStore {
    lfs: LocalFileSystem,
}

impl LocalDataStore {
    pub fn new(path: PathBuf) -> Result<Self, OnDiskError> {
        Ok(LocalDataStore {
            lfs: LocalFileSystem::new_with_prefix(path)
                .map_err(|err| OnDiskError::Implementation(err.to_string()))?,
        })
    }
}

fn cid_as_path(cid: &Cid) -> Path {
    Path::parse(&format!("/{}.bin", cid.as_base64url_multicodec())).unwrap()
}

#[async_trait(?Send)]
impl DataStore for LocalDataStore {
    async fn contains_cid(&self, cid: Cid) -> Result<bool, DataStoreError> {
        match self.lfs.head(&cid_as_path(&cid)).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { path: _, source: _ }) => Ok(false),
            Err(_) => Err(DataStoreError::LookupFailure),
        }
    }

    async fn remove(&mut self, cid: Cid, _recusrive: bool) -> Result<(), DataStoreError> {
        self.lfs
            .delete(&cid_as_path(&cid))
            .await
            .map_err(|_| DataStoreError::StoreFailure)
    }

    async fn retrieve(&self, cid: Cid) -> Result<Vec<u8>, DataStoreError> {
        let result = self
            .lfs
            .get(&cid_as_path(&cid))
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
        self.lfs
            .put(&cid_as_path(&cid), data.into())
            .await
            .map_err(|_| DataStoreError::StoreFailure)
            .map(|_| ())
    }
}

/*
#[async_trait(?Send)]
impl OnDisk<String> for OnDiskDataStore {
    const TYPE: DiskType = DiskType::Config;
    const SUFFIX: &'static str = "data_stores";
    const EXTENSION: &'static str = "ds";

    async fn encode(&self, identifier: &String) -> Result<(), OnDiskError> {
        let fix = self.lfs.prefix;
        Ok(())
    }

    async fn decode(identifier: &String) -> Result<Self, OnDiskError> {
        Ok(Self {

        })
    }
}
*/
