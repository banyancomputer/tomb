mod api;
mod carv2_disk;
mod carv2_memory;
mod error;
mod memory;
mod multi_carv2_disk;
mod split;

pub use api::BanyanApiBlockStore;
pub use carv2_disk::CarV2DiskBlockStore;
pub use carv2_memory::CarV2MemoryBlockStore;
pub use error::BlockStoreError;
pub use memory::MemoryBlockStore;
pub use multi_carv2_disk::MultiCarV2DiskBlockStore;
pub use split::DoubleSplitStore;

use async_trait::async_trait;
/// Wrap a BlockStore with additional functionality to get / set a root CID
#[async_trait(?Send)]
pub trait RootedBlockStore: BanyanBlockStore {
    /// Get the root CID
    fn get_root(&self) -> Option<Cid>;
    /// Set the root CID
    fn set_root(&self, root: &Cid);
}

/*

#[async_trait(?Send)]
pub trait BanyanBlockStore: wnfs::common::BlockStore {
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid, BlockStoreError>;
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, BlockStoreError>;
}

*/
