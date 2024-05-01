use std::fmt::Display;

use tokio::fs::{File, OpenOptions};

use crate::on_disk::{DataType, DiskData, DiskDataError};
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

#[async_trait(?Send)]
impl DiskData<DriveAndKeyId> for Drive {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: &'static str = "drives";
    const EXTENSION: &'static str = "bfs";

    async fn encode(&self, identifier: &DriveAndKeyId) -> Result<(), DiskDataError> {
        self.encode(
            &mut crypto_rng(),
            ContentOptions::everything(),
            &mut Self::get_writer(identifier).await?,
        )
        .await?;
        Ok(())
    }

    async fn decode(identifier: &DriveAndKeyId) -> Result<Self, DiskDataError> {
        let user_key = SigningKey::decode(&identifier.user_key_id).await?;
        let drive = DriveLoader::new(&user_key)
            .from_reader(&mut Self::get_reader(identifier).await?)
            .await
            .map_err(|err| DiskDataError::Implementation(err.to_string()))?;
        Ok(drive)
    }
}
