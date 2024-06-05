use crate::{
    on_disk::{DiskType, OnDisk, OnDiskError},
    ConfigStateError, NativeError,
};
use async_trait::async_trait;
use banyanfs::{
    api::{platform, ApiClient},
    codec::crypto::SigningKey,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::info;
use uuid::Uuid;

/// Represents the Global contents of the tomb configuration file in a user's .config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    /// Banyan-Cli version
    version: String,
    /// User Key Identifier of Key in Use
    selected_user_key_id: Option<String>,
    /// User Key Identifiers
    user_key_ids: Vec<String>,
    /// Drive Identifiers/Names -> Disk Locations
    drive_paths: HashMap<String, PathBuf>,
    /// Drive Identifiers -> Platform Drive Identifiers
    drive_platform_ids: HashMap<String, String>,
    /// Drive Previous Metadata ID
    pub(crate) drive_previous_metadata_ids: HashMap<String, String>,
    /// Platform account id
    account_id: Option<Uuid>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            selected_user_key_id: None,
            user_key_ids: vec![],
            drive_paths: HashMap::new(),
            drive_platform_ids: HashMap::new(),
            drive_previous_metadata_ids: HashMap::new(),
            account_id: None,
        }
    }
}

impl GlobalConfig {
    pub async fn get_client(&self) -> Result<ApiClient, NativeError> {
        let account_id = self.get_account_id()?.to_string();
        let key = Arc::new(SigningKey::decode(&self.selected_user_key_id()?).await?);
        Ok(ApiClient::new(env!("ENDPOINT"), &account_id, key)
            .map_err(|_| OnDiskError::Implementation("Api Client creation".to_string()))?)
    }

    pub fn select_user_key_id(&mut self, user_key_id: String) {
        self.selected_user_key_id = Some(user_key_id);
    }

    pub fn deselect_user_key_id(&mut self) {
        self.selected_user_key_id = None;
    }

    pub fn selected_user_key_id(&self) -> Result<String, ConfigStateError> {
        self.selected_user_key_id
            .clone()
            .ok_or(ConfigStateError::NoKeySelected)
    }

    pub fn set_path(&mut self, drive_id: &str, path: &Path) {
        self.drive_paths
            .insert(drive_id.to_string(), path.to_path_buf());
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

    pub fn set_account_id(&mut self, account_id: &str) -> Result<(), NativeError> {
        self.account_id = Some(Uuid::parse_str(account_id)?);
        Ok(())
    }

    pub fn get_account_id(&self) -> Result<Uuid, ConfigStateError> {
        self.account_id.ok_or(ConfigStateError::NoAccountId)
    }

    pub fn remove_account_id(&mut self) {
        self.account_id = None;
    }

    pub async fn drive_platform_id(&mut self, drive_id: &str) -> Result<String, NativeError> {
        if let Some(platform_id) = self.drive_platform_ids.get(drive_id) {
            return Ok(platform_id.to_string());
        }
        let client = self.get_client().await?;
        let drive_platform_id = platform::drives::get_all(&client)
            .await?
            .into_iter()
            .find(|drive| drive.name == drive_id)
            .ok_or(ConfigStateError::MissingDrive(drive_id.into()))?
            .id;
        self.drive_platform_ids
            .insert(drive_id.to_string(), drive_platform_id.clone());
        self.encode(&GlobalConfigId).await?;
        Ok(drive_platform_id)
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
        let mut writer = Self::get_std_writer(identifier)?;
        serde_json::to_writer_pretty(&mut writer, &self)?;
        info!("<< PREFERENCE SAVED >>");
        Ok(())
    }
    async fn decode(identifier: &GlobalConfigId) -> Result<Self, OnDiskError> {
        let mut reader = Self::get_std_reader(identifier)?;
        Ok(serde_json::from_reader(&mut reader)?)
    }
}
