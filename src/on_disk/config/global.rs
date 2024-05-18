use crate::{
    on_disk::{DiskType, OnDisk, OnDiskError},
    ConfigStateError, NativeError,
};
use async_trait::async_trait;
use banyanfs::{api::ApiClient, codec::crypto::SigningKey};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};
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
    /// Remote account id
    account_id: Option<Uuid>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            selected_user_key_id: None,
            user_key_ids: vec![],
            drive_paths: HashMap::new(),
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
        tracing::info!("<< PREFERENCE SAVED >>");
        Ok(())
    }
    async fn decode(identifier: &GlobalConfigId) -> Result<Self, OnDiskError> {
        let mut reader = File::open(Self::path(identifier)?)?;
        Ok(serde_json::from_reader(&mut reader)?)
    }
}
