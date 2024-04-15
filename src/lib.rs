//! This crate contains all modules in our project. TODO(organizedgrime) write something useful here.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]

/// CLI Parsing
//#[cfg(feature = "cli")]
//pub mod cli;

// we can just use banyanfs platform
//pub(crate) mod api;

// we dont need car filing in the client anymore i dont think
//pub(crate) mod car;

// we do want a custom disk based datastore
pub(crate) mod drive;

// we can just use the banyanfs
//pub(crate) mod filesystem;
pub mod native;
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
