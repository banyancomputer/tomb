use crate::blockstore::{BlockStore, RootedBlockStore};
use crate::car::v1::block::Block;
use crate::car::v2::CarV2;
use anyhow::Result;
use async_trait::async_trait;
use std::cell::RefCell;
use std::{borrow::Cow, io::Cursor};
use wnfs::libipld::{Cid, IpldCodec};

#[derive(Debug)]
/// CarV2 formatted memory blockstore
pub struct CarV2MemoryBlockStore {
    data: RefCell<Vec<u8>>,
    car: CarV2,
}

impl TryFrom<Vec<u8>> for CarV2MemoryBlockStore {
    type Error = anyhow::Error;

    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        let mut rw = Cursor::new(&vec);
        let car = CarV2::read_bytes(&mut rw)?;
        let data = RefCell::new(vec);
        Ok(Self { data, car })
    }
}

impl CarV2MemoryBlockStore {
    /// Create a new CarV2BlockStore from a readable stream
    pub fn new() -> Result<Self> {
        // Read data
        let vec = Vec::new();
        let mut rw = Cursor::new(vec);
        let car = CarV2::new(&mut rw)?;
        // Wrap the vec in a RefCell and add it to self
        let data = RefCell::new(rw.into_inner());
        Ok(Self { data, car })
    }

    /// Get a reader to the data underlying the CarV2
    pub fn get_data(&self) -> Vec<u8> {
        self.car.to_bytes().unwrap()
    }
}

#[async_trait(?Send)]
/// WnfsBlockStore implementation for CarV2BlockStore
impl BlockStore for CarV2MemoryBlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, anyhow::Error> {
        let vec = self.data.borrow();
        let mut reader = Cursor::new(&*vec);
        let block = self.car.get_block(cid, &mut reader)?;
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, content: Vec<u8>, codec: IpldCodec) -> Result<Cid, anyhow::Error> {
        let mut vec = self.data.borrow_mut();
        let mut writer = Cursor::new(&mut *vec);
        let block = Block::new(content, codec)?;
        self.car.put_block(&block, &mut writer)?;
        Ok(block.cid)
    }
}

#[async_trait(?Send)]
/// RootedBlockStore implementation for CarV2BlockStore -- needed in order to interact with the Fs
impl RootedBlockStore for CarV2MemoryBlockStore {
    fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }

    fn set_root(&self, root: &Cid) {
        self.car.set_root(root)
    }

    // async fn update_block(&self, _: &Cid, _: Vec<u8>, _: IpldCodec) -> Result<Cid, anyhow::Error> {
    //     panic!("update block deprecated / not implemented")
    // }
}
