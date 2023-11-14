use super::CarV2DiskBlockStore;
use crate::{
    api::requests::staging::upload::content::{ContentType, UploadContent},
    blockstore::{BlockStore, BlockStoreError, RootedBlockStore},
    car::error::CarError,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs::{self, create_dir_all};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};
use wnfs::libipld::{Cid, IpldCodec};

/// CARv2 MultiCarV2DiskBlockStore across multiple CAR files using File IO
#[derive(Debug, PartialEq, Clone)]
pub struct MultiCarV2DiskBlockStore {
    /// CAR directory path
    pub path: PathBuf,
    /// CARv2 BlockStores inside this BlockStore
    pub deltas: Vec<CarV2DiskBlockStore>,
}

impl MultiCarV2DiskBlockStore {
    /// Create a new CARv2 MultifileMultiCarV2DiskBlockStore from a directory
    pub fn new(dir: &Path) -> Result<Self, BlockStoreError> {
        if dir.is_file() {
            Err(BlockStoreError::expected_directory(dir))
        } else {
            // If the folder doesn't already exist
            if !dir.exists() {
                // Make it
                create_dir_all(dir)?;
            }

            let mut deltas = Vec::new();
            // For each child in the directory
            for dir_entry in fs::read_dir(dir)? {
                // If the dir entry is valid, the file is a .car, and a MultiCarV2DiskBlockStore can be read from it
                if let Ok(entry) = dir_entry
                    && entry
                        .file_name()
                        .to_str()
                        .expect("no file name str")
                        .contains(".car")
                    && let Ok(car) = CarV2DiskBlockStore::new(&entry.path())
                {
                    // Push this to the vec
                    deltas.push(car);
                }
            }

            // Sort so that the most recent delta is last in the list
            deltas.sort_by(|a, b| a.path.cmp(&b.path));

            // Ok
            Ok(Self {
                path: dir.to_path_buf(),
                deltas,
            })
        }
    }

    /// Add a new delta file / CAR file
    pub fn add_delta(&mut self) -> Result<(), BlockStoreError> {
        // Create a new delta for writing
        let new_store =
            CarV2DiskBlockStore::new(&self.path.join(format!("{}.car", self.deltas.len() + 1)))?;

        // Set the root depending on previous deltas
        if !self.deltas.is_empty() {
            new_store.set_root(
                &self
                    .get_delta()?
                    .get_root()
                    .ok_or(BlockStoreError::car(CarError::missing_root()))?,
            );
        } else {
            new_store.set_root(&Cid::default());
        }

        // Add the new store
        self.deltas.push(new_store);

        // Ok
        Ok(())
    }

    /// Get the most recent delta
    pub fn get_delta(&self) -> Result<&CarV2DiskBlockStore, BlockStoreError> {
        self.deltas.last().ok_or(BlockStoreError::no_such_file())
    }
}

#[async_trait(?Send)]
impl BlockStore for MultiCarV2DiskBlockStore {
    async fn get_block(&self, cid: &Cid) -> anyhow::Result<Cow<'_, Vec<u8>>> {
        // Iterate in reverse order
        for store in self.deltas.iter().rev() {
            // If block is retrieved
            if let Ok(data) = store.get_block(cid).await {
                // Ok
                return Ok(data);
            }
        }

        // We didn't find the CID in any BlockStore
        Err(BlockStoreError::car(CarError::missing_block(cid)).into())
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> anyhow::Result<Cid> {
        // If there is a delta
        let current_delta = self.get_delta()?;
        let cid = current_delta.put_block(bytes, codec).await?;
        Ok(cid)
    }
}

#[async_trait(?Send)]
impl RootedBlockStore for MultiCarV2DiskBlockStore {
    fn get_root(&self) -> Option<Cid> {
        if let Ok(current_delta) = self.get_delta() {
            current_delta.get_root()
        } else {
            None
        }
    }

    fn set_root(&self, root: &Cid) {
        if !self.deltas.is_empty() {
            let current_delta = self.get_delta().unwrap();
            current_delta.set_root(root);
            current_delta.to_disk().expect("failed to write to disk");
        }
    }
}

#[async_trait(?Send)]
impl UploadContent for MultiCarV2DiskBlockStore {
    type UploadError = anyhow::Error;

    fn get_hash(&self) -> Result<String, Self::UploadError> {
        let reader = std::fs::File::open(&self.get_delta()?.path)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update_reader(&reader)?;
        Ok(hasher.finalize().to_string())
    }

    #[allow(refining_impl_trait)]
    async fn get_body(&self) -> Result<ContentType, Self::UploadError> {
        Ok(tokio::fs::File::open(&self.get_delta()?.path).await?.into())
    }

    fn get_length(&self) -> Result<u64, Self::UploadError> {
        Ok(self.get_delta()?.path.metadata()?.len())
    }
}

impl Serialize for MultiCarV2DiskBlockStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.path.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MultiCarV2DiskBlockStore {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let path = PathBuf::deserialize(deserializer)?;
        Self::new(&path).map_err(|err| {
            serde::de::Error::custom(format!("MultiCARv2 Deserialization error: {err}"))
        })
    }
}

#[cfg(test)]
mod test {

    use serial_test::serial;
    use std::{fs::remove_dir_all, path::Path};
    use wnfs::{
        common::{
            blockstore::{bs_duplication_test, bs_retrieval_test},
            BlockStore,
        },
        libipld::IpldCodec,
    };

    use crate::blockstore::{BlockStoreError, MultiCarV2DiskBlockStore};

    #[tokio::test]
    #[serial]
    async fn multidelta() -> Result<(), BlockStoreError> {
        let path = &Path::new("test").join("multidelta");
        // Delete this if it exists
        if path.exists() {
            remove_dir_all(path)?;
        }

        let mut store = MultiCarV2DiskBlockStore::new(path)?;
        // Create a new delta
        store.add_delta()?;

        // Assert that there are now two delta CARs
        assert_eq!(store.deltas.len(), 1);

        let hello_kitty = "Hello Kitty!".as_bytes().to_vec();
        let hello_kitty_cid = store.put_block(hello_kitty.clone(), IpldCodec::Raw).await?;

        // Create a new delta
        store.add_delta()?;
        // Assert that there are now two delta CARs
        assert_eq!(store.deltas.len(), 2);

        let goodbye_kitty = "Goodbye Kitty!".as_bytes().to_vec();
        let goodbye_kitty_cid = store
            .put_block(goodbye_kitty.clone(), IpldCodec::Raw)
            .await?;

        // Assert that both blocks are still retrievable, despite being in separate CAR files
        assert_eq!(
            store.get_block(&hello_kitty_cid).await?.to_vec(),
            hello_kitty
        );
        assert_eq!(
            store.get_block(&goodbye_kitty_cid).await?.to_vec(),
            goodbye_kitty
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn unidelta() -> Result<(), BlockStoreError> {
        let path = &Path::new("test").join("unidelta");
        // Delete this if it exists
        if path.exists() {
            remove_dir_all(path)?;
        }

        let mut store = MultiCarV2DiskBlockStore::new(path)?;

        // Create a new delta
        store.add_delta()?;
        // Assert that there is one CAR
        assert_eq!(store.deltas.len(), 1);

        let hello_kitty = "Hello Kitty!".as_bytes().to_vec();
        let hello_kitty_cid = store.put_block(hello_kitty.clone(), IpldCodec::Raw).await?;

        let goodbye_kitty = "Goodbye Kitty!".as_bytes().to_vec();
        let goodbye_kitty_cid = store
            .put_block(goodbye_kitty.clone(), IpldCodec::Raw)
            .await?;

        // Assert that both blocks are still retrievable, despite being in separate CAR files
        assert_eq!(
            store.get_block(&hello_kitty_cid).await?.to_vec(),
            hello_kitty
        );
        assert_eq!(
            store.get_block(&goodbye_kitty_cid).await?.to_vec(),
            goodbye_kitty
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn multifileblockstore() -> Result<(), BlockStoreError> {
        let test_dir = &Path::new("test").join("car").join("multifile_blockstore");
        if test_dir.exists() {
            remove_dir_all(test_dir)?;
        }
        let mut store = MultiCarV2DiskBlockStore::new(test_dir)?;
        store.add_delta()?;
        bs_retrieval_test(&store).await?;
        bs_duplication_test(&store).await?;
        Ok(())
    }
}
