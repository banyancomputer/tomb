use super::*;
use crate::{drive::DiskDrive, utils::get_read, NativeError};
use banyanfs::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{remove_file, OpenOptions},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use url::Url;
use uuid::Uuid;

/// Represents the Global contents of the tomb configuration file in a user's .config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    /// Banyan-Cli version
    version: String,

    /// Location of wrapping key on disk in PEM format
    pub user_key_path: PathBuf,

    /// Remote endpoint
    endpoint: Url,
    /// Remote account id
    account_id: Option<Uuid>,
    /// Drive Configurations
    pub(crate) drives: HashMap<String, DriveConfig>,
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
            user_key_path: default_user_key_path(),
            account_id: None,
            drives: HashMap::new(),
        }
    }
}

// Self
impl GlobalConfig {
    /// Create a new Global Configuration, keys, and save them all
    pub async fn new() -> Result<Self, NativeError> {
        // Create a default config
        let config = Self::default();
        config.to_disk()?;

        // Do not blindly overwrite key files if they exist
        if !config.user_key_path.exists() {
            create(&config.user_key_path).await?;
        }
        // Ok
        Ok(config)
    }

    /// Get the user key
    pub async fn user_key(&self) -> Result<SigningKey, NativeError> {
        save(&self.user_key_path)
            .await
            .map_err(|_| NativeError::missing_api_key())
    }

    /*
    // Get the Gredentials
    async fn get_credentials(&self) -> Result<Credentials, NativeError> {
        Ok(Credentials {
            user_id: self.remote_user_id.ok_or(NativeError::missing_user_id())?,
            signing_key: self.user_key().await?,
        })
    }
    */

    /// Get the ApiClient data
    pub async fn get_client(&self) -> Result<ApiClient, NativeError> {
        if let Some(account_id) = self.account_id {
            let user_key = Arc::new(self.user_key().await?);
            // Create a new ApiClient
            ApiClient::new(
                &self.endpoint.to_string(),
                &account_id.to_string(),
                user_key,
            )
            .map_err(|err| NativeError::custom_error(&format!("client creation: {err}")))
        } else {
            return Err(NativeError::custom_error("no account id in config"));
        }
    }

    #[allow(unused)]
    pub fn get_endpoint(&self) -> Url {
        self.endpoint.clone()
    }

    pub fn set_endpoint(&mut self, endpoint: Url) -> Result<(), NativeError> {
        self.endpoint = endpoint;
        self.to_disk()
    }

    /// Write to disk
    fn to_disk(&self) -> Result<(), NativeError> {
        let writer = OpenOptions::new()
            .create(true)
            .append(false)
            .truncate(true)
            .write(true)
            .open(config_path())?;

        serde_json::to_writer_pretty(writer, &self).map_err(|_| NativeError::bad_data())
    }

    /// Initialize from file on disk
    pub async fn from_disk() -> Result<Self, NativeError> {
        let file = get_read(&config_path())?;
        let config = serde_json::from_reader(file).map_err(|_| NativeError::bad_data())?;
        Ok(config)
    }

    /// Remove a BucketConfig for an origin
    pub fn remove_drive(&mut self, bucket: &LocalDrive) -> Result<(), NativeError> {
        // Remove bucket data
        bucket.remove_data()?;
        // Find index of bucket
        let index = self
            .drives
            .iter()
            .position(|b| b == bucket)
            .expect("cannot find index in buckets");
        // Remove bucket config from global config
        self.drives.remove(index);
        self.to_disk()
    }

    /// Remove Config data associated with each Bucket
    pub fn remove_all_data(&self) -> Result<(), NativeError> {
        // Remove bucket data
        for bucket in &self.drives {
            bucket.remove_data()?;
        }
        // Remove global
        let path = config_path();
        if path.exists() {
            remove_file(path)?;
        }
        self.to_disk()
    }

    /// Update a given BucketConfig
    pub fn update_config(&mut self, drive: &LocalDrive) -> Result<(), NativeError> {
        // Find index
        let index = self
            .drives
            .iter()
            .position(|b| b.origin == drive.origin)
            .ok_or(NativeError::missing_local_drive())?;
        // Update bucket at index
        self.drives[index] = drive.clone();
        self.to_disk()
    }

    /// Create a new bucket
    async fn create_drive(&mut self, name: &str, origin: &Path) -> Result<LocalDrive, NativeError> {
        let user_key = self.user_key().await?;
        let mut bucket = LocalDrive::new(origin, &user_key).await?;
        bucket.name = name.to_string();
        self.drives.push(bucket.clone());
        self.to_disk()?;
        Ok(bucket)
    }

    /// Get a Bucket configuration by the origin
    pub fn get_drive(&self, origin: &Path) -> Option<LocalDrive> {
        self.drives
            .iter()
            .find(|drive| drive.origin == origin)
            .cloned()
    }

    /// Create a bucket if it doesn't exist, return the object either way
    pub async fn get_or_init_drive(
        &mut self,
        name: &str,
        origin: &Path,
    ) -> Result<LocalDrive, NativeError> {
        if let Some(config) = self.get_drive(origin) {
            Ok(config.clone())
        } else {
            Ok(self.create_drive(name, origin).await?)
        }
    }
}

#[cfg(test)]
mod test {

    use serial_test::serial;
    use std::{fs::remove_file, path::Path};

    use crate::native::{
        configuration::{
            globalconfig::GlobalConfig,
            xdg::{config_path, default_user_key_path},
        },
        NativeError,
    };

    #[tokio::test]
    #[serial]
    async fn to_from_disk() -> Result<(), NativeError> {
        // The known path of the global config file
        let known_path = config_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        let known_path = default_user_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        // Create default
        let original = GlobalConfig::new().await?;
        // Load from disk
        let reconstructed = GlobalConfig::from_disk().await?;
        assert_eq!(original, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn from_disk_direct() -> Result<(), NativeError> {
        // The known path of the global config file
        let known_path = config_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        let known_path = default_user_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        // Load from disk
        let reconstructed = GlobalConfig::new().await?;
        let known_path = default_user_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        assert_eq!(GlobalConfig::from_disk().await?, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn add_bucket() -> Result<(), NativeError> {
        // The known path of the global config file
        let known_path = config_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }

        let origin = Path::new("test");

        // Create
        let mut original = GlobalConfig::new().await?;
        let original_bucket = original.get_or_init_drive("new", origin).await?;
        // Save
        original.to_disk()?;
        let reconstructed = GlobalConfig::from_disk().await?;
        let reconstructed_bucket = reconstructed
            .get_drive(origin)
            .expect("bucket config does not exist for this origin");

        // Assert equality
        assert_eq!(original_bucket.metadata, reconstructed_bucket.metadata);
        assert_eq!(original_bucket.content, reconstructed_bucket.content);

        Ok(())
    }
}
