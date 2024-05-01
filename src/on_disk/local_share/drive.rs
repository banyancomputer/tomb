use tokio::fs::{File, OpenOptions};

use async_trait::async_trait;
use banyanfs::{
    codec::{crypto::SigningKey, header::ContentOptions},
    filesystem::{Drive, DriveLoader},
    utils::crypto_rng,
};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::on_disk::{DataType, DiskData, DiskDataError};

#[async_trait(?Send)]
impl DiskData for Drive {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: &'static str = "drives";
    const EXTENSION: &'static str = "bfs";

    //async fn encode(&self, identifier: String) {
    async fn encode(&self, identifier: &str) -> Result<(), DiskDataError> {
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
    async fn decode(identifier: &str) -> Result<Self, DiskDataError> {
        let path = Self::path(identifier)?;
        let mut fh = File::open(path).await.unwrap().compat();
        let user_key = SigningKey::decode("owner".into()).await.unwrap();
        let drive = DriveLoader::new(&user_key)
            .from_reader(&mut fh)
            .await
            .unwrap();

        Ok(drive)
    }
}
