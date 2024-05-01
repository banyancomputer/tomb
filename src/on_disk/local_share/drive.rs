use std::fmt::Display;

use tokio::fs::{File, OpenOptions};

use async_trait::async_trait;
use banyanfs::{
    codec::{crypto::SigningKey, header::ContentOptions},
    filesystem::{Drive, DriveLoader},
    utils::crypto_rng,
};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::on_disk::{DataType, DiskData, DiskDataError};

//
#[derive(Debug)]
pub struct DriveId {
    pub drive_id: String,
    pub user_key_id: String,
}
impl Display for DriveId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.drive_id))
    }
}

#[async_trait(?Send)]
impl DiskData<DriveId> for Drive {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: &'static str = "drives";
    const EXTENSION: &'static str = "bfs";

    //async fn encode(&self, identifier: String) {
    async fn encode(&self, identifier: &DriveId) -> Result<(), DiskDataError> {
        let path = Self::path(identifier)?;
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
    async fn decode(identifier: &DriveId) -> Result<Self, DiskDataError> {
        let path = Self::path(identifier)?;
        let mut fh = File::open(path).await.unwrap().compat();
        let user_key = SigningKey::decode(&identifier.user_key_id).await.unwrap();
        let drive = DriveLoader::new(&user_key)
            .from_reader(&mut fh)
            .await
            .map_err(|err| DiskDataError::Implementation(err.to_string()))?;
        Ok(drive)
    }
}
