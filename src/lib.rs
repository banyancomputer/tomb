//! This crate contains all modules in our project. TODO(organizedgrime) write something useful here.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]

/// CLI Parsing
//#[cfg(feature = "cli")]
//pub mod cli;
//pub(crate) mod api;
//pub(crate) mod car;
pub(crate) mod datastore;
//pub(crate) mod filesystem;
//pub mod native;
pub(crate) mod utils;

pub mod prelude {
    /*
    pub mod api {
        pub use crate::api::{client, models, requests};
    }
    pub mod blockstore {
        pub use crate::blockstore::{
            BanyanApiBlockStore, BanyanBlockStore, CarV2DiskBlockStore, CarV2MemoryBlockStore,
            DoubleSplitStore, MemoryBlockStore, MultiCarV2DiskBlockStore, RootedBlockStore,
        };
    }
    */
    /*
    pub mod car {
        pub use crate::car::{v1, v2};
    }
    pub mod filesystem {
        pub use crate::filesystem::{serialize, sharing, wnfsio, FilesystemError, FsMetadata};
    }
    */
}
