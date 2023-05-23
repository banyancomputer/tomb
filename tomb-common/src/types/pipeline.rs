use crate::types::blockstore::carblockstore::CarBlockStore;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// This is the struct that becomes the contents of the manifest file.
/// It may seem silly to have a struct that has only one field, but in
/// versioning this struct, we can also version its children identically.
/// As well as any other fields we may add / remove in the future.
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// The project version that was used to encode this Manifest
    pub version: String,
    /// The BlockStore that holds all packed data
    pub content_store: CarBlockStore,
    /// The BlockStore that holds all Metadata
    pub meta_store: CarBlockStore,
}

impl Debug for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Manifest")
            .field("version", &self.version)
            .finish()
    }
}
