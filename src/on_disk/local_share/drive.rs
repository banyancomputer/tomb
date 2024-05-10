use std::fmt::Display;

use tokio::fs::{File, OpenOptions};

use crate::on_disk::{DiskType, OnDisk, OnDiskError};
use async_trait::async_trait;
use banyanfs::{
    codec::{crypto::SigningKey, header::ContentOptions},
    filesystem::{Drive, DriveLoader},
    utils::crypto_rng,
};

//
#[derive(Debug)]
pub struct DriveAndKeyId {
    pub drive_id: String,
    pub user_key_id: String,
}
impl Display for DriveAndKeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.drive_id))
    }
}

/// ~/.local/share/banyan/drives
/// Contains .bfs files representing drives
#[async_trait(?Send)]
impl OnDisk<DriveAndKeyId> for Drive {
    const TYPE: DiskType = DiskType::LocalShare;
    const SUFFIX: &'static str = "drives";
    const EXTENSION: &'static str = "bfs";

    async fn encode(&self, identifier: &DriveAndKeyId) -> Result<(), OnDiskError> {
        self.encode(
            &mut crypto_rng(),
            ContentOptions::everything(),
            &mut Self::get_writer(identifier).await?,
        )
        .await?;
        Ok(())
    }

    async fn decode(identifier: &DriveAndKeyId) -> Result<Self, OnDiskError> {
        let user_key = SigningKey::decode(&identifier.user_key_id).await?;
        let drive = DriveLoader::new(&user_key)
            .from_reader(&mut Self::get_reader(identifier).await?)
            .await
            .map_err(|err| OnDiskError::Implementation(err.to_string()))?;
        Ok(drive)
    }
}
