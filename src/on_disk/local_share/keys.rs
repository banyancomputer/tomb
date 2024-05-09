use super::super::*;
use async_trait::async_trait;
use banyanfs::prelude::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

#[async_trait(?Send)]
impl DiskData<String> for SigningKey {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: &'static str = "user_keys";
    const EXTENSION: &'static str = "pem";

    async fn encode(&self, identifier: &String) -> Result<(), DiskDataError> {
        let pem: String = self.to_pkcs8_pem().unwrap().to_string();
        Self::get_writer(identifier)
            .await?
            .compat_write()
            .write_all(pem.as_bytes())
            .await?;
        return Ok(());
    }

    async fn decode(identifier: &String) -> Result<Self, DiskDataError> {
        let mut pem_bytes = Vec::new();
        Self::get_reader(identifier)
            .await?
            .compat()
            .read_to_end(&mut pem_bytes)
            .await?;
        let pem = String::from_utf8(pem_bytes).unwrap();
        let key = SigningKey::from_pkcs8_pem(&pem).unwrap();
        return Ok(key);
    }
}
