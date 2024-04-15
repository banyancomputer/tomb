//#[cfg(feature = "cli")]
use crate::cli::specifiers::DriveSpecifier;
use crate::{
    api::models::drive::{Drive as RemoteDrive, DriveType, StorageClass},
    native::{
        configuration::globalconfig::GlobalConfig,
        sync::{LocalDrive, SyncState},
        NativeError,
    },
};
use colored::{ColoredString, Colorize};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};
use tomb_crypt::prelude::{PrivateKey, PublicKey};
use uuid::Uuid;

/// Struct for representing the ambiguity between local and remote copies of a Drive
#[derive(Debug, Clone)]
pub struct OmniDrive {
    /// The local Drive
    local: Option<LocalDrive>,
    /// The remote Drive
    remote: Option<RemoteDrive>,
    /// The sync state
    pub sync_state: SyncState,
}

impl OmniDrive {
    /// Use local and remote to find
    //#[cfg(feature = "cli")]
    pub async fn from_specifier(drive_specifier: &DriveSpecifier) -> Self {
        let mut omni = Self {
            local: None,
            remote: None,
            sync_state: SyncState::Unknown,
        };

        if let Ok(global) = GlobalConfig::from_disk().await {
            let local_result = global.drives.clone().into_iter().find(|drive| {
                let check_remote = drive.remote_id == drive_specifier.drive_id;
                let check_origin = Some(drive.origin.clone()) == drive_specifier.origin;
                let check_name = Some(drive.name.clone()) == drive_specifier.name;
                check_remote || check_origin || check_name
            });
            omni.local = local_result;

            if let Ok(mut client) = global.get_client().await {
                let all_remote_drives = RemoteDrive::read_all(&mut client)
                    .await
                    .unwrap_or(Vec::new());
                let remote_result = all_remote_drives.into_iter().find(|drive| {
                    let check_id = Some(drive.id) == drive_specifier.drive_id;
                    let check_name = Some(drive.name.clone()) == drive_specifier.name;
                    check_id || check_name
                });
                omni.remote = remote_result;

                if omni.local.is_some() && omni.remote.is_some() {
                    let mut local = omni.get_local().unwrap();
                    local.remote_id = Some(omni.get_remote().unwrap().id);
                    omni.local = Some(local);
                }

                // Determine the sync state
                let _ = omni.determine_sync_state().await;
            }
        }

        omni
    }

    /// Initialize w/ local
    pub fn from_local(drive: &LocalDrive) -> Self {
        Self {
            local: Some(drive.clone()),
            remote: None,
            sync_state: SyncState::Unpublished,
        }
    }

    /// Initialize w/ remote
    pub fn from_remote(drive: &RemoteDrive) -> Self {
        Self {
            local: None,
            remote: Some(drive.clone()),
            sync_state: SyncState::Unlocalized,
        }
    }

    /// Get the ID from wherever it might be found
    pub fn get_id(&self) -> Result<Uuid, NativeError> {
        if let Some(remote) = self.remote.clone() {
            Ok(remote.id)
        } else if let Some(local) = self.local.clone() {
            local.remote_id.ok_or(NativeError::missing_identifier())
        } else {
            Err(NativeError::missing_identifier())
        }
    }

    /// Get the local config
    pub fn get_local(&self) -> Result<LocalDrive, NativeError> {
        self.local.clone().ok_or(NativeError::missing_local_drive())
    }

    /// Get the remote config
    pub fn get_remote(&self) -> Result<RemoteDrive, NativeError> {
        self.remote
            .clone()
            .ok_or(NativeError::missing_remote_drive())
    }

    /// Update the LocalDrive
    pub fn set_local(&mut self, local: LocalDrive) {
        self.local = Some(local);
    }

    /// Update the RemoteDrive
    pub fn set_remote(&mut self, remote: RemoteDrive) {
        self.remote = Some(remote);
    }

    /// Create a new drive
    pub async fn create(name: &str, origin: &Path) -> Result<OmniDrive, NativeError> {
        let mut global = GlobalConfig::from_disk().await?;

        let mut omni = OmniDrive {
            local: None,
            remote: None,
            sync_state: SyncState::Unknown,
        };

        // If this drive already exists both locally and remotely
        if let Some(drive) = global.get_drive(origin) {
            if drive.remote_id.is_some() {
                // Prevent the user from re-creating it
                return Err(NativeError::unique_error());
            }
        }

        // Grab the wrapping key, public key and pem
        let wrapping_key = global.wrapping_key().await?;
        let public_key = wrapping_key.public_key()?;
        let pem = String::from_utf8(public_key.export().await?)?;

        // Initialize remotely
        if let Ok((remote, _)) = RemoteDrive::create(
            name.to_string(),
            pem,
            DriveType::Interactive,
            StorageClass::Hot,
            &mut global.get_client().await?,
        )
        .await
        {
            // Update in obj
            omni.set_remote(remote);
        }

        // Initialize locally
        if let Ok(mut local) = global.get_or_init_drive(name, origin).await {
            // If a remote drive was made successfully
            if let Ok(remote) = omni.get_remote() {
                // Also save that in the local obj
                local.remote_id = Some(remote.id);
            }
            // Update in global and obj
            global.update_config(&local.clone())?;
            omni.local = Some(local);
        }

        Ok(omni)
    }

    /// Delete an individual Drive
    pub async fn delete(
        &self,
        local_deletion: bool,
        mut remote_deletion: bool,
    ) -> Result<String, NativeError> {
        let mut global = GlobalConfig::from_disk().await?;
        if local_deletion {
            global.remove_drive(&self.get_local()?)?;
        }

        if remote_deletion {
            remote_deletion =
                RemoteDrive::delete_by_id(&mut global.get_client().await?, self.get_remote()?.id)
                    .await
                    .is_ok();
        }

        Ok(format!(
            "{}\ndeleted locally:\t{}\ndeleted remotely:\t{}",
            "<< BUCKET DELETION >>".blue(),
            bool_colorized(local_deletion),
            bool_colorized(remote_deletion)
        ))
    }

    /// List all available Drives
    pub async fn ls() -> Result<Vec<OmniDrive>, NativeError> {
        let mut client = GlobalConfig::from_disk().await?.get_client().await?;
        let local_drives = if let Ok(global) = GlobalConfig::from_disk().await {
            global.drives
        } else {
            Vec::new()
        };
        let remote_drives = match RemoteDrive::read_all(&mut client).await {
            Ok(drives) => drives,
            Err(_) => {
                error!(
                    "{}",
                    "Unable to fetch remote Drives. Check your authentication!".red()
                );
                <Vec<RemoteDrive>>::new()
            }
        };

        let mut map: HashMap<Option<Uuid>, OmniDrive> = HashMap::new();

        for local in local_drives {
            map.insert(local.remote_id, OmniDrive::from_local(&local));
        }

        for remote in remote_drives {
            let key = Some(remote.id);
            if let Some(omni) = map.get(&key) {
                let mut omni = OmniDrive {
                    local: omni.local.clone(),
                    remote: Some(remote),
                    sync_state: SyncState::Unknown,
                };

                omni.determine_sync_state().await?;

                map.insert(key, omni);
            } else {
                map.insert(key, OmniDrive::from_remote(&remote));
            }
        }

        Ok(map.into_values().collect::<Vec<OmniDrive>>())
    }

    /// Get the origin for this drive or create one in the default tomb directory if a local drive does not yet exist
    pub async fn get_or_init_origin(&mut self) -> Result<PathBuf, NativeError> {
        if let Ok(local) = self.get_local() {
            Ok(local.origin)
        } else {
            let new_local_origin = PathBuf::from(env!("HOME"))
                .join("tomb")
                .join(self.get_remote()?.name);
            // Remove existing contents and create a enw directory
            remove_dir_all(&new_local_origin).ok();
            create_dir_all(&new_local_origin)?;

            // Create a new local drive
            self.set_local({
                let mut value = GlobalConfig::from_disk()
                    .await?
                    .get_or_init_drive(&self.get_remote()?.name, &new_local_origin)
                    .await?;
                value.remote_id = Some(self.get_remote()?.id);
                value
            });

            Ok(new_local_origin)
        }
    }

    /// Unlock FsMetadata
    pub async fn unlock(&self) -> Result<FsMetadata, NativeError> {
        let local = self.get_local()?;
        let global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.wrapping_key().await?;
        FsMetadata::unlock(&wrapping_key, &local.metadata)
            .await
            .map_err(NativeError::filesytem)
    }
}

#[inline]
fn bool_colorized(value: bool) -> ColoredString {
    if value {
        "Yes".green()
    } else {
        "No".red()
    }
}

impl Display for OmniDrive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut info = format!(
            "{}\nlocally tracked:\t{}\nremotely tracked:\t{}",
            "| DRIVE INFO |".yellow(),
            bool_colorized(self.local.is_some()),
            bool_colorized(self.remote.is_some()),
        );

        match (self.get_local(), self.get_remote()) {
            (Ok(local), Ok(remote)) => {
                info = format!(
                    "{info}\nname:\t\t\t{}\ndrive_id:\t\t{}\norigin:\t\t\t{}\ntype:\t\t\t{}\nstorage_class:\t\t{}\nstorage_ticket:\t\t{}",
                    remote.name,
                    remote.id,
                    local.origin.display(),
                    remote.r#type,
                    remote.storage_class,
                    if let Some(storage_ticket) = local.storage_ticket.clone() {
                        storage_ticket.host
                    } else {
                        format!("{}", "None".yellow())
                    }
                );
            }
            (Ok(local), Err(_)) => {
                info = format!("{info}\n{}", local);
            }
            (Err(_), Ok(remote)) => {
                info = format!("{info}\n{}", remote);
            }
            (Err(_), Err(_)) => {}
        }

        f.write_fmt(format_args!(
            "{info}\nsync_status:\t\t{}\n",
            self.sync_state
        ))
    }
}
