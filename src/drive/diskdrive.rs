use super::datastore::DiskDataStore;
use crate::on_disk::*;
use async_trait::async_trait;
use banyanfs::prelude::*;
use banyanfs::{codec::crypto::SigningKey, error::BanyanFsResult, utils::crypto_rng};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs::create_dir_all, path::PathBuf, sync::Arc};
use tokio::fs::{File, OpenOptions};
use tokio_util::compat::TokioAsyncReadCompatExt;

pub struct DiskDrive {
    store: DiskDataStore,
    drive: Drive,
}

/*
impl DiskDrive {
    async fn new(name: String, user_key: Arc<SigningKey>) -> BanyanFsResult<Self> {
        // Determine the paths we'll be working from
        let root = xdg_data_home().join(name);
        let lfs_root = root.join("store");

        // Error out if duplicate
        if root.exists() {
            return Err(String::from("drive with this name already exists").into());
        }

        // Create directory and new store
        create_dir_all(&lfs_root).map_err(|e| e.to_string())?;
        let store = DiskDataStore::new_at_path(lfs_root.to_string_lossy().to_string())
            .map_err(|e| e.to_string())?;

        // Create an initial
        let mut rng = crypto_rng();
        let drive =
            Drive::initialize_private(&mut rng, user_key.clone()).map_err(|e| e.to_string())?;
        let mut disk_drive = Self { root, store, drive };
        disk_drive.write();
        disk_drive.read(user_key);
        Ok(disk_drive)
    }
}

*/

#[async_trait(?Send)]
impl DiskData for Drive {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: &'static str = "drives";
    const EXTENSION: &'static str = "bfs";

    //async fn encode(&self, identifier: String) {
    async fn encode(&self, identifier: String) -> Result<(), DiskDataError> {
        let path = Self::path(identifier);
        let mut rng = crypto_rng();
        let mut file_opts = OpenOptions::new();
        file_opts.write(true);
        file_opts.create(true);
        file_opts.truncate(true);

        let mut fh = file_opts.open(path).await.unwrap().compat();

        self.encode(&mut rng, ContentOptions::everything(), &mut fh)
            .await
            .unwrap();
        Ok(())
    }

    //async fn read(&mut self, user_key: Arc<SigningKey>) {
    async fn decode(identifier: String) -> Result<Self, DiskDataError> {
        let path = Self::path(identifier);
        let mut fh = File::open(path).await.unwrap().compat();
        let user_key = SigningKey::decode("owner".into()).await.unwrap();
        let drive = DriveLoader::new(&user_key)
            .from_reader(&mut fh)
            .await
            .unwrap();

        Ok(drive)
    }
}
