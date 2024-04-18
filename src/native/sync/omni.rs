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

impl OmniDrive {
    /// Determine the Sync State of an omni bucket
    pub async fn determine_sync_state(&mut self) -> Result<(), NativeError> {
        let bucket_id = match self.get_id() {
            Ok(bucket_id) => bucket_id,
            Err(err) => {
                info!("err: {}", err);
                self.sync_state = SyncState::Unpublished;
                return Ok(());
            }
        };

        // Grab the current remote Metadata, or return Unpublished if that operation fails
        let mut client = GlobalConfig::from_disk().await?.get_client().await?;
        let Ok(current_remote) = Metadata::read_current(bucket_id, &mut client).await else {
            self.sync_state = SyncState::Unpublished;
            return Ok(());
        };
        // Grab the local bucket, or return Unlocalized if unavailable
        if let Ok(local) = self.get_local() {
            let local_metadata_cid = local.metadata.get_root().map(|cid| cid.to_string());
            let local_content_cid = local.content.get_root().map(|cid| cid.to_string());
            // If the metadata root CIDs match
            if local_metadata_cid == Some(current_remote.metadata_cid) {
                // If the block is also persisted locally in content
                if local_content_cid == Some(current_remote.root_cid) {
                    self.sync_state = SyncState::AllSynced
                } else {
                    self.sync_state = SyncState::MetadataSynced;
                }
                Ok(())
            } else {
                let all_metadatas = Metadata::read_all(bucket_id, &mut client).await?;
                // If the current Metadata id exists in the list of remotely persisted ones
                if all_metadatas
                    .iter()
                    .any(|metadata| Some(metadata.metadata_cid.clone()) == local_metadata_cid)
                {
                    self.sync_state = SyncState::Behind;
                    Ok(())
                } else {
                    self.sync_state = SyncState::Ahead;
                    Ok(())
                }
            }
        } else {
            self.sync_state = SyncState::Unlocalized;
            Ok(())
        }
    }

    /// Sync
    #[allow(unused)]
    pub async fn sync_bucket(&mut self) -> Result<String, NativeError> {
        let mut global = GlobalConfig::from_disk().await?;
        let mut client = global.get_client().await?;
        match &self.sync_state {
            // Download the Bucket
            SyncState::Unlocalized | SyncState::Behind => {
                let current = Metadata::read_current(self.get_id()?, &mut client).await?;
                let mut byte_stream = current.pull(&mut client).await?;

                self.get_or_init_origin().await.ok();

                let mut buffer = <Vec<u8>>::new();
                // Write every chunk to it
                while let Some(chunk) = byte_stream.next().await {
                    tokio::io::copy(&mut chunk.map_err(ApiError::http)?.as_ref(), &mut buffer)
                        .await?;
                }
                // Attempt to create a CARv2 BlockStore from the data
                let metadata = CarV2MemoryBlockStore::try_from(buffer)?;
                // Grab the metadata file
                let mut metadata_file =
                    tokio::fs::File::create(&self.get_local()?.metadata.path).await?;
                metadata_file.write_all(&metadata.get_data()).await?;
                // Write that data out to the metadatas

                info!("{}", "<< METADATA RECONSTRUCTED >>".green());
                self.sync_state = SyncState::MetadataSynced;
                Ok(format!(
                    "{}",
                    "<< DATA STILL NOT DOWNLOADED; SYNC AGAIN >>".blue()
                ))
            }
            // Upload the Bucket
            SyncState::Unpublished | SyncState::Ahead => {
                let mut local = self.get_local()?;
                let wrapping_key = global.wrapping_key().await?;
                let fs = local.unlock_fs(&wrapping_key).await?;

                // If there is still no ID, that means the remote Bucket was never created
                if self.get_id().is_err() {
                    let public_key = wrapping_key.public_key()?;
                    let pem = String::from_utf8(public_key.export().await?)?;
                    let (remote, _) = Bucket::create(
                        local.name.clone(),
                        pem,
                        BucketType::Interactive,
                        StorageClass::Hot,
                        &mut client,
                    )
                    .await?;

                    self.set_remote(remote.clone());
                    local.remote_id = Some(remote.id);
                    global.update_config(&local)?;
                    self.set_local(local.clone());
                }

                // Extract variables or error
                let bucket_id = self.get_id()?;
                let local_content_cid = local
                    .content
                    .get_root()
                    .ok_or(FilesystemError::missing_metadata("root cid"))?;
                let local_metadata_cid = local
                    .metadata
                    .get_root()
                    .ok_or(FilesystemError::missing_metadata("metdata cid"))?;
                let delta = local.content.get_delta()?;

                // Push the metadata
                let (metadata, host, authorization) = Metadata::push(
                    PushMetadata {
                        bucket_id,
                        expected_data_size: delta.data_size(),
                        root_cid: local_content_cid.to_string(),
                        metadata_cid: local_metadata_cid.to_string(),
                        previous_cid: local.previous_cid.map(|cid| cid.to_string()),
                        valid_keys: fs.share_manager.public_fingerprints(),
                        deleted_block_cids: local
                            .deleted_block_cids
                            .clone()
                            .iter()
                            .map(|v| v.to_string())
                            .collect(),
                        metadata_stream: tokio::fs::File::open(&local.metadata.path).await?.into(),
                    },
                    &mut client,
                )
                .await?;

                // Empty the list of deleted blocks, now that it's the server's problem
                local.deleted_block_cids = BTreeSet::new();

                if host.is_none() && authorization.is_none() {
                    local.storage_ticket = None;
                }

                info!("Uploading your new data now...");

                let upload_result = match (host, authorization) {
                    // New storage ticket
                    (Some(host), Some(authorization)) => {
                        // Update the storage ticket locally and create grant
                        let storage_ticket = StorageTicket {
                            host,
                            authorization,
                        };
                        storage_ticket.create_grant(&mut client).await?;
                        local.storage_ticket = Some(storage_ticket.clone());
                        local
                            .content
                            .upload(storage_ticket.host, metadata.id, &mut client)
                            .await
                    }
                    // Already granted, still upload
                    (Some(host), None) => {
                        local.content.upload(host, metadata.id, &mut client).await
                    }
                    // No uploading required
                    _ => {
                        global.update_config(&local)?;
                        self.set_local(local);
                        return Ok("METADATA PUSHED; NO CONTENT PUSH NEEDED".to_string());
                    }
                };

                global.update_config(&local)?;
                self.set_local(local);

                match upload_result {
                    // Upload succeeded
                    Ok(()) => {
                        self.sync_state = SyncState::AllSynced;
                        Metadata::read_current(bucket_id, &mut client)
                            .await
                            .map(|new_metadata| {
                                format!(
                                    "{}\n{}",
                                    "<< SUCCESSFULLY UPLOADED METADATA & CONTENT >>".green(),
                                    new_metadata
                                )
                            })
                            .map_err(NativeError::api)
                    }
                    // Upload failed
                    Err(_) => Ok(format!(
                        "{}\n{}\n{}\n",
                        "<< FAILED TO PUSH CONTENT >>".red(),
                        "<< SUCCESSFULLY PUSHED PENDING METADATA >>".green(),
                        metadata
                    )),
                }
            }
            // Reconstruct the Bucket locally
            SyncState::MetadataSynced => {
                let local = self.get_local()?;
                let api_blockstore_client = client.clone();
                let mut api_blockstore = BanyanApiBlockStore::from(api_blockstore_client);
                let metadata_root_cid = local
                    .metadata
                    .get_root()
                    .ok_or(FilesystemError::missing_metadata("root cid"))?;
                let mut cids = BTreeSet::new();
                cids.insert(metadata_root_cid);
                api_blockstore.find_cids(cids).await?;
                // If getting a block is an error
                if api_blockstore.get_block(&metadata_root_cid).await.is_err() {
                    // Grab storage host
                    let storage_host = local
                        .clone()
                        .storage_ticket
                        .map(|ticket| ticket.host)
                        .ok_or(NativeError::custom_error(
                            "unable to determine storage host",
                        ))?;
                    // Get authorization
                    let authorization = self.get_remote()?.get_grants_token(&mut client).await?;
                    // Create a grant for this Client so that future BlockStore calls will succeed
                    let storage_ticket = StorageTicket {
                        host: storage_host,
                        authorization,
                    };
                    storage_ticket.create_grant(&mut client).await?;
                }

                // Open the FileSystem
                let fs = FsMetadata::unlock(&global.wrapping_key().await?, &local.metadata).await?;
                // Reconstruct the data on disk
                let restoration_result = restore::pipeline(self.clone()).await;
                // If we succeed at reconstructing
                if restoration_result.is_ok() {
                    // Save the metadata in the content store as well
                    let metadata_cid = local.metadata.get_root().unwrap();
                    let ipld = local
                        .metadata
                        .get_deserializable::<Ipld>(&metadata_cid)
                        .await
                        .map_err(Box::from)?;
                    let content_cid = local
                        .content
                        .put_serializable(&ipld)
                        .await
                        .map_err(Box::from)?;
                    local.content.set_root(&content_cid);
                    assert_eq!(metadata_cid, content_cid);
                    // We're now all synced up
                    self.sync_state = SyncState::AllSynced;
                }

                info!("{self}");
                restoration_result
            }
            SyncState::AllSynced => Ok(format!(
                "{}",
                "This Bucket data is already synced :)".green()
            )),
            SyncState::Unknown => {
                self.determine_sync_state().await?;
                Ok(format!(
                    "{}",
                    format!("<< SYNC STATE UPDATED TO {:?} >>", self.sync_state).blue()
                ))
            }
        }
    }
}
