use super::*;
use crate::on_disk::{DataType, DiskData, DiskDataError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs::File;
use url::Url;
use uuid::Uuid;

/// Represents the Global contents of the tomb configuration file in a user's .config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    /// Banyan-Cli version
    version: String,
    /// User Key Identifiers
    user_key_ids: Vec<String>,
    /// Drive Identifiers
    drive_ids: Vec<String>,
    /// Remote endpoint
    endpoint: Url,
    /// Remote account id
    account_id: Option<Uuid>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let endpoint = Url::parse(if option_env!("DEV_ENDPOINTS").is_some() {
            "http://127.0.0.1:3001"
        } else {
            "https://beta.data.banyan.computer"
        })
        .expect("unable to parse known URLs");

        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            endpoint,
            ..Default::default()
        }
    }
}

#[async_trait(?Send)]
impl DiskData for GlobalConfig {
    const TYPE: DataType = DataType::Config;
    const SUFFIX: &'static str = "";
    const EXTENSION: &'static str = "json";

    async fn encode(&self, identifier: &str) -> Result<(), DiskDataError> {
        let mut writer = File::create(Self::path(identifier)?)?;
        serde_json::to_writer_pretty(&mut writer, &self)?;
        Ok(())
    }
    async fn decode(identifier: &str) -> Result<Self, DiskDataError> {
        let mut reader = File::open(Self::path(identifier)?)?;
        Ok(serde_json::from_reader(&mut reader)?)
    }
}
