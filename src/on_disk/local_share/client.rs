use async_trait::async_trait;
use banyanfs::{
    api::ApiClient,
    codec::{crypto::SigningKey, header::ContentOptions},
    filesystem::{Drive, DriveLoader},
    utils::crypto_rng,
};
use tokio::fs::{File, OpenOptions};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::on_disk::{DataType, DiskData, DiskDataError};

/*
#[async_trait(?Send)]
impl DiskData for ApiClient {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: &'static str = "";
    const EXTENSION: &'static str = "client";

    //async fn encode(&self, identifier: String) {
    async fn encode(&self, identifier: &str) -> Result<(), DiskDataError> {
        let path = Self::path(identifier);
        Ok(())
    }

    //async fn read(&mut self, user_key: Arc<SigningKey>) {
    async fn decode(identifier: &str) -> Result<Self, DiskDataError> {
        let path = Self::path(identifier);
        Ok(drive)
    }
}
*/
