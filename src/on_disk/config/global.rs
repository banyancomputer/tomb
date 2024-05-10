use crate::on_disk::{DiskType, OnDisk, OnDiskError};
use async_trait::async_trait;
use banyanfs::{api::ApiClient, codec::crypto::SigningKey};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs::File, sync::Arc};
use uuid::Uuid;

/// Represents the Global contents of the tomb configuration file in a user's .config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    /// Banyan-Cli version
    version: String,
    /// User Key Identifier of Key in Use
    pub selected_user_key_id: Option<String>,
    /// User Key Identifiers
    user_key_ids: Vec<String>,
    /// Drive Identifiers
    pub drive_ids: Vec<String>,
    /// Remote account id
    account_id: Option<Uuid>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            selected_user_key_id: None,
            user_key_ids: vec![],
            drive_ids: vec![],
            account_id: None,
        }
    }
}

impl GlobalConfig {
    pub async fn api_client(&self) -> Result<ApiClient, OnDiskError> {
        let suki = self
            .selected_user_key_id
            .clone()
            .ok_or(OnDiskError::Implementation(
                "No user key selected".to_string(),
            ))?;
        let account_id = self
            .account_id
            .ok_or(OnDiskError::Implementation("No account id".to_string()))?
            .to_string();
        let key = Arc::new(SigningKey::decode(&suki).await?);
        Ok(ApiClient::new(env!("ENDPOINT"), &account_id, key)
            .map_err(|_| OnDiskError::Implementation("Api Client creation".to_string()))?)
    }

    /*
    pub async fn selected_key(&self) -> Result<SigningKey, OnDiskError> {
        let key_id = self
            .selected_user_key_id
            .clone()
            .ok_or(OnDiskError::Implementation(
                "No user key selected".to_string(),
            ))?;
        SigningKey::decode(&key_id).await
    }
    */
}

pub struct GlobalConfigId;
impl Display for GlobalConfigId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("global")
    }
}

#[async_trait(?Send)]
impl OnDisk<GlobalConfigId> for GlobalConfig {
    const TYPE: DiskType = DiskType::Config;
    const SUFFIX: &'static str = "";
    const EXTENSION: &'static str = "json";

    // TODO async serde_json?

    async fn encode(&self, identifier: &GlobalConfigId) -> Result<(), OnDiskError> {
        let mut writer = File::create(Self::path(identifier)?)?;
        serde_json::to_writer_pretty(&mut writer, &self)?;
        Ok(())
    }
    async fn decode(identifier: &GlobalConfigId) -> Result<Self, OnDiskError> {
        let mut reader = File::open(Self::path(identifier)?)?;
        Ok(serde_json::from_reader(&mut reader)?)
    }
}
