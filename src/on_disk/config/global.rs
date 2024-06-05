use crate::{
    on_disk::{DiskType, OnDisk, OnDiskError},
    ConfigStateError, NativeError,
};
use async_trait::async_trait;
use banyanfs::{api::ApiClient, codec::crypto::SigningKey};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, path::PathBuf, sync::Arc};
use tracing::info;
use url::Url;
use uuid::Uuid;

/// Represents the Global contents of the tomb configuration file in a user's .config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    /// Banyan-Cli version
    version: String,
    /// User Key Identifier of Key in Use
    pub(crate) selected_key_id: Option<String>,
    /// User Key Identifiers
    user_key_ids: Vec<String>,
    /// Drive Identifiers/Names -> Disk Locations
    pub(crate) drive_paths: HashMap<String, PathBuf>,
    /// Drive Identifiers -> Platform Drive Identifiers
    pub(crate) drive_platform_ids: HashMap<String, String>,
    /// Drive Previous Metadata ID
    pub(crate) drive_previous_metadata_ids: HashMap<String, String>,
    /// Cached storage grants
    pub(crate) storage_grants: HashMap<Url, String>,
    /// Platform account id
    pub(crate) account_id: Option<Uuid>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            selected_key_id: None,
            user_key_ids: vec![],
            drive_paths: HashMap::new(),
            drive_platform_ids: HashMap::new(),
            drive_previous_metadata_ids: HashMap::new(),
            storage_grants: HashMap::new(),
            account_id: None,
        }
    }
}

impl GlobalConfig {
    pub async fn get_client(&self) -> Result<ApiClient, NativeError> {
        let account_id = self.get_account_id()?.to_string();
        let key = Arc::new(SigningKey::decode(&self.selected_key_id()?).await?);
        let client = ApiClient::new(env!("ENDPOINT"), &account_id, key)
            .map_err(|_| OnDiskError::Implementation("Api Client creation".to_string()))?;
        for (host, grant) in self.storage_grants.iter() {
            client.record_storage_grant(host.clone(), grant).await;
        }
        Ok(client)
    }

    pub fn selected_key_id(&self) -> Result<String, ConfigStateError> {
        self.selected_key_id
            .clone()
            .ok_or(ConfigStateError::NoKeySelected)
    }

    pub fn get_path(&self, drive_id: &str) -> Result<PathBuf, ConfigStateError> {
        self.drive_paths
            .get(drive_id)
            .cloned()
            .ok_or(ConfigStateError::MissingDrive(drive_id.to_string()))
    }

    pub fn remove_path(&mut self, drive_id: &str) -> Result<PathBuf, ConfigStateError> {
        self.drive_paths
            .remove(drive_id)
            .ok_or(ConfigStateError::MissingDrive(drive_id.to_string()))
    }

    pub fn get_account_id(&self) -> Result<Uuid, ConfigStateError> {
        self.account_id.ok_or(ConfigStateError::NoAccountId)
    }
}

pub struct GlobalConfigId;
impl Display for GlobalConfigId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("global")
    }
}

/// ~/.config/banyan
/// Contains a single global.json
#[async_trait(?Send)]
impl OnDisk<GlobalConfigId> for GlobalConfig {
    const TYPE: DiskType = DiskType::Config;
    const SUFFIX: &'static str = "";
    const EXTENSION: &'static str = "json";

    async fn encode(&self, identifier: &GlobalConfigId) -> Result<(), OnDiskError> {
        let mut writer = Self::sync_writer(identifier)?;
        serde_json::to_writer_pretty(&mut writer, &self)?;
        info!("<< PREFERENCE SAVED >>");
        Ok(())
    }
    async fn decode(identifier: &GlobalConfigId) -> Result<Self, OnDiskError> {
        let mut reader = Self::sync_reader(identifier)?;
        Ok(serde_json::from_reader(&mut reader)?)
    }
}
