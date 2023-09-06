use futures_util::StreamExt;
use js_sys::{Array, ArrayBuffer, Uint8Array};
use std::convert::TryFrom;
use std::io::Cursor;
use tomb_common::banyan_api::blockstore::BanyanApiBlockStore;
use tomb_common::banyan_api::client::Client;
use tomb_common::banyan_api::models::snapshot::Snapshot;
use tomb_common::banyan_api::models::{bucket::Bucket, bucket_key::BucketKey, metadata::Metadata};
use tomb_common::blockstore::carv2_memory::CarV2MemoryBlockStore as BlockStore;
use tomb_common::metadata::FsMetadata;
use tomb_crypt::prelude::*;
use wasm_bindgen::prelude::*;

use crate::log;

// TODO: This should be a config
const BLOCKSTORE_API_HOST: &str = "http://127.0.0.1:3002";

use crate::error::TombWasmError;
use crate::types::{WasmBucket, WasmFsMetadataEntry, WasmSnapshot};
use crate::JsResult;

/// Mount point for a Bucket in WASM
///
/// Enables to call Fs methods on a Bucket, pulling metadata from a remote
#[wasm_bindgen]
pub struct WasmMount {
    client: Client,

    bucket: Bucket,

    /// Currently initialized version of Fs Metadata
    metadata: Option<Metadata>,

    locked: bool,

    /// Whether or not a change requires a call to save
    dirty: bool,

    /// Whether or not data has been appended to the content blockstore
    append: bool,

    metadata_blockstore: BlockStore,
    content_blockstore: BlockStore,

    fs_metadata: Option<FsMetadata>,
}

impl WasmMount {
    /// Initialize a new Wasm callable mount with metadata for a bucket and a client
    pub async fn new(
        wasm_bucket: WasmBucket,
        key: &EcEncryptionKey,
        client: &Client,
    ) -> Result<Self, TombWasmError> {
        log!("tomb-wasm: mount/new()/{}", wasm_bucket.id());

        let bucket = Bucket::from(wasm_bucket.clone());
        log!(
            "tomb-wasm: mount/new()/{} - creating blockstores",
            wasm_bucket.id()
        );
        let metadata_blockstore = BlockStore::new().expect("could not create blockstore");
        let content_blockstore = BlockStore::new().expect("could not create blockstore");
        log!(
            "tomb-wasm: mount/new()/{} - creating fs metadata",
            wasm_bucket.id()
        );
        let fs_metadata = FsMetadata::init(key)
            .await
            .expect("could not init fs metadata");
        log!(
            "tomb-wasm: mount/new()/{} - saving fs metadata",
            wasm_bucket.id()
        );
        let mut mount = Self {
            client: client.to_owned(),
            bucket,
            metadata: None,
            locked: false,
            dirty: true,
            append: false,

            metadata_blockstore,
            content_blockstore,
            fs_metadata: Some(fs_metadata),
        };

        log!("tomb-wasm: mount/new()/{} - syncing", wasm_bucket.id());
        mount.sync().await.expect("could not sync");
        // Ok
        Ok(mount)
    }
    /// Initialize a new Wasm callable mount with metadata for a bucket and a client
    pub async fn pull(wasm_bucket: WasmBucket, client: &mut Client) -> Result<Self, TombWasmError> {
        log!("tomb-wasm: mount/pull()/{}", wasm_bucket.id());
        // Get the underlying bucket
        let bucket = Bucket::from(wasm_bucket.clone());

        // Get the metadata associated with the bucket
        let metadata = Metadata::read_current(bucket.id, client)
            .await
            .map_err(|err| TombWasmError(format!("unable to read current metadata: {err}")))?;

        let metadata_cid = metadata.metadata_cid.clone();
        log!(
            "tomb-wasm: mount/pull()/{} - pulling metadata at version {}",
            wasm_bucket.id(),
            metadata_cid
        );
        // Pull the Fs metadata on the matching entry
        let mut stream = metadata
            .pull(client)
            .await
            .expect("could not pull metedata");
        log!(
            "tomb-wasm: mount/pull()/{} - reading metadata stream",
            wasm_bucket.id()
        );
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        log!(
            "tomb-wasm: mount/pull()/{} - creating metadata blockstore",
            wasm_bucket.id()
        );
        let metadata_blockstore =
            BlockStore::try_from(data).expect("could not create metadata as blockstore");
        let content_blockstore = BlockStore::new().expect("could not create blockstore");

        log!("tomb-wasm: mount/pull()/{} - pulled", wasm_bucket.id());

        // Ok
        Ok(Self {
            client: client.to_owned(),
            bucket,
            metadata: Some(metadata.to_owned()),
            locked: true,
            dirty: false,
            append: false,

            metadata_blockstore,
            content_blockstore,
            fs_metadata: None,
        })
    }

    /// Refresh the current fs_metadata with the remote
    pub async fn refresh(&mut self, key: &EcEncryptionKey) -> Result<(), TombWasmError> {
        let bucket_id = self.bucket.id;

        // Get the metadata associated with the bucket
        let metadata = Metadata::read_current(bucket_id, &mut self.client)
            .await
            .map_err(|err| TombWasmError(format!("failed to read current metadata: {err}")))?;

        let metadata_cid = metadata.metadata_cid.clone();
        log!(
            "tomb-wasm: mount/pull()/{} - pulling metadata at version {}",
            self.bucket.id.to_string(),
            metadata_cid
        );

        // Pull the Fs metadata on the matching entry
        let mut stream = metadata
            .pull(&mut self.client)
            .await
            .expect("could not pull metedata");

        log!(
            "tomb-wasm: mount/pull()/{} - reading metadata stream",
            self.bucket.id.to_string()
        );

        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }

        log!(
            "tomb-wasm: mount/pull()/{} - creating metadata blockstore",
            self.bucket.id.to_string()
        );

        let metadata_blockstore =
            BlockStore::try_from(data).expect("could not create metadata as blockstore");
        let content_blockstore = BlockStore::new().expect("could not create blockstore");

        self.metadata = Some(metadata.to_owned());
        self.metadata_blockstore = metadata_blockstore;
        self.content_blockstore = content_blockstore;
        self.dirty = false;
        self.append = false;
        self.fs_metadata = None;

        log!(
            "tomb-wasm: mount/pull()/{} - pulled",
            self.bucket.id.to_string()
        );
        self.unlock(key).await.expect("could not unlock");
        // Ok
        Ok(())
    }

    /// Sync the current fs_metadata with the remote
    pub async fn sync(&mut self) -> Result<(), TombWasmError> {
        log!("tomb-wasm: mount/sync()/{}", self.bucket.id.to_string());
        // Check if the bucket is locked
        if self.locked() {
            log!(
                "tomb-wasm: mount/sync()/{} - bucket is locked",
                self.bucket.id.to_string()
            );
            panic!("Bucket is locked");
        };
        log!(
            "tomb-wasm: mount/sync()/{} - saving changes",
            self.bucket.id.to_string()
        );

        if self.dirty() {
            log!(
                "tomb-wasm: mount/sync()/{} - saving changes to fs",
                self.bucket.id.to_string()
            );
            let _ = self
                .fs_metadata
                .as_mut()
                .unwrap()
                .save(&self.metadata_blockstore, &self.content_blockstore)
                .await;
        } else {
            log!(
                "tomb-wasm: mount/sync()/{} - no changes to fs",
                self.bucket.id.to_string()
            );
        }

        log!(
            "tomb-wasm: mount/sync()/{} - pushing changes",
            self.bucket.id.to_string()
        );

        let root_cid = self
            .fs_metadata
            .as_ref()
            .unwrap()
            .root_cid(&self.metadata_blockstore)
            .await
            .expect("could not get root cid");
        let metadata_cid = self
            .fs_metadata
            .as_ref()
            .unwrap()
            .metadata_cid(&self.metadata_blockstore)
            .await
            .expect("could not get metadata cid");

        log!(
            "tomb-wasm: mount/sync()/{} - pushing metadata at version {}",
            self.bucket.id.to_string(),
            metadata_cid.to_string()
        );
        log!(format!(
            "tomb-wasm: mount/sync()/{} - pushing root at version {}",
            self.bucket.id, root_cid,
        ));
        // Assume that the metadata is always at least as big as the content
        let mut data_size = 0;
        if self.append {
            data_size = self.content_blockstore.data_size();
        }
        log!(
            "tomb-wasm: mount/sync()/{} - content size difference {data_size}",
            self.bucket.id.to_string(),
            metadata_cid.to_string(),
            data_size
        );
        let (metadata, storage_ticket) = Metadata::push(
            self.bucket.id,
            root_cid.to_string(),
            metadata_cid.to_string(),
            data_size,
            // This may lint as an error but it is not
            Cursor::new(self.metadata_blockstore.get_data()),
            &mut self.client,
        )
        .await
        .expect("could not push metadata");

        assert_eq!(metadata.metadata_cid, metadata_cid.to_string());
        assert_eq!(metadata.root_cid, root_cid.to_string());
        let metadata_id = metadata.id;
        self.metadata = Some(metadata);

        match storage_ticket {
            Some(storage_ticket) => {
                log!(
                    "tomb-wasm: mount/sync()/ - storage ticket returned",
                    self.bucket.id.to_string()
                );

                storage_ticket
                    .clone()
                    .create_grant(&mut self.client)
                    .await
                    .map_err(|err| {
                        TombWasmError(format!("unable to register storage ticket: {err}"))
                    })?;

                let content = Cursor::new(self.metadata_blockstore.get_data());
                storage_ticket
                    .clone()
                    .upload_content(metadata_id, content, &mut self.client)
                    .await
                    .map_err(|err| {
                        TombWasmError(format!(
                            "unable to upload data to distribution service: {err}"
                        ))
                    })?;
            }
            None => {
                log!(format!(
                    "tomb-wasm: mount/sync()/{} - no storage ticket returned no content to upload",
                    self.bucket.id,
                ));
            }
        }

        self.dirty = false;
        self.append = false;

        log!(format!(
            "tomb-wasm: mount/sync()/{} - synced",
            self.bucket.id.to_string()
        ));

        Ok(())
    }

    /// Unlock the current fs_metadata
    pub async fn unlock(&mut self, key: &EcEncryptionKey) -> Result<(), TombWasmError> {
        log!(format!("tomb-wasm: mount/unlock()/{}", self.bucket.id));

        // Check if the bucket is already unlocked
        if !self.locked() {
            return Ok(());
        }

        log!(format!(
            "tomb-wasm: mount/unlock()/{} - unlocking",
            self.bucket.id,
        ));

        // Get the metadata
        let fs_metadata = FsMetadata::unlock(key, &self.metadata_blockstore)
            .await
            .map_err(|err| TombWasmError(format!("could not unlock fs metadata: {err}")))?;

        log!(format!(
            "tomb-wasm: mount/unlock()/{} - checking versioning",
            self.bucket.id,
        ));

        let metadata_cid = fs_metadata
            .metadata_cid(&self.metadata_blockstore)
            .await
            .map_err(|err| TombWasmError(format!("unable to retrieve metadata CID: {err}")))?;

        let root_cid = fs_metadata
            .root_cid(&self.metadata_blockstore)
            .await
            .map_err(|err| TombWasmError(format!("unable to retrieve root CID: {err}")))?;

        let metadata = self.metadata.as_ref().unwrap();

        assert_eq!(metadata_cid.to_string(), metadata.metadata_cid);
        assert_eq!(root_cid.to_string(), metadata.root_cid);

        log!(format!(
            "tomb-wasm: mount/unlock()/{} - unlocked",
            self.bucket.id,
        ));

        self.locked = false;
        self.fs_metadata = Some(fs_metadata);

        Ok(())
    }
}

#[wasm_bindgen]
impl WasmMount {
    /// Returns whether or not the bucket is locked
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// Returns whether or not the bucket is dirty
    /// - when a file or dir is changed
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Ls the bucket at a path
    /// # Arguments
    /// * `path_segments` - The path to ls (as an Array)
    /// # Returns
    /// The an Array of objects in the form of:
    /// This is an instance of
    /// ```json
    /// [
    /// 0.{
    ///    "name": "string",
    ///   "entry_type": "string", (file | dir)
    ///  "metadata": {
    ///    "created": 0,
    ///   "modified": 0,
    ///  "size": 0,
    /// "cid": "string"
    /// }
    /// ]
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    pub async fn ls(&mut self, path_segments: Array) -> JsResult<Array> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/ls/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            return Err(TombWasmError(
                "unable to list directory contents of a locked bucket".to_string(),
            )
            .into());
        };

        log!(format!(
            "tomb-wasm: mount/ls/{}/{} - getting entries",
            self.bucket.id,
            &path_segments.join("/")
        ));

        // Get the entries
        let fs_metadata_entries = self
            .fs_metadata
            .as_ref()
            .unwrap()
            .ls(path_segments, &self.metadata_blockstore)
            .await
            .map_err(|err| TombWasmError(format!("could not list directory entries: {err}")))?;

        log!(format!(
            "tomb-wasm: mount/ls/{} - mapping entries",
            self.bucket.id,
        ));

        // Map the entries back to JsValues
        fs_metadata_entries
            .iter()
            .map(|entry| {
                let wasm_fs_metadata_entry = WasmFsMetadataEntry::from(entry.clone());
                JsValue::try_from(wasm_fs_metadata_entry).map_err(|err| {
                    TombWasmError(format!(
                        "unable to convert directory entries to JS objects: {err:?}"
                    ))
                    .into()
                })
            })
            .collect()
    }

    /// Mkdir
    /// # Arguments
    /// * `path_segments` - The path to mkdir (as an Array)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not mkdir` - If the mkdir fails
    /// * `Could not sync` - If the sync fails
    pub async fn mkdir(&mut self, path_segments: Array) -> JsResult<()> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/mkdir/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        log!(
            "tomb-wasm: mount/mkdir/{}/{} - mkdir",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );
        self.fs_metadata
            .as_mut()
            .unwrap()
            .mkdir(path_segments, &self.metadata_blockstore)
            .await
            .expect("could not mkdir");

        log!(
            "tomb-wasm: mount/mkdir/{}/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    /// Add a file
    /// # Arguments
    /// * `path_segments` - The path to add to (as an Array)
    /// * `content_buffer` - The content to add (as an ArrayBuffer)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not add` - If the add fails
    /// * `Could not sync` - If the sync fails
    pub async fn add(&mut self, path_segments: Array, content_buffer: ArrayBuffer) -> JsResult<()> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/add/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        let content = Uint8Array::new(&content_buffer).to_vec();

        self.fs_metadata
            .as_mut()
            .unwrap()
            .add(
                path_segments,
                content,
                &self.metadata_blockstore,
                &self.content_blockstore,
            )
            .await
            .expect("could not add");
        log!(
            "tomb-wasm: mount/add/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.append = true;
        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    /// Read a file from a mounted bucket
    ///     Read / Download a File (takes a path to a file inside the bucket, not available for cold only buckets)
    ///     Allows reading at a version
    /// # Arguments
    /// * `path_segments` - The path to read from (as an Array)
    /// * `version` - The version to read from (optional)
    /// # Returns
    /// A Promise<ArrayBuffer> in js speak
    #[wasm_bindgen(js_name = readBytes)]
    pub async fn read_bytes(
        &mut self,
        path_segments: Array,
        _version: Option<String>,
    ) -> JsResult<ArrayBuffer> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/read_bytes/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        let mut banyan_api_blockstore_client = self.client.clone();
        banyan_api_blockstore_client
            .with_remote(BLOCKSTORE_API_HOST)
            .expect("could not create blockstore client");
        let banyan_api_blockstore = BanyanApiBlockStore::from(banyan_api_blockstore_client);

        let vec = self
            .fs_metadata
            .as_mut()
            .unwrap()
            .read(
                path_segments,
                &self.metadata_blockstore,
                &banyan_api_blockstore,
            )
            .await
            .expect("could not read bytes");

        let bytes = vec.into_boxed_slice();
        let array = Uint8Array::from(&bytes[..]);
        Ok(array.buffer())
    }

    // TODO: Get metadata on node

    /// Mv a file or directory
    /// # Arguments
    /// * `from_path_segments` - The path to mv from (as an Array)
    /// * `to_path_segments` - The path to mv to (as an Array)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not mv` - If the mv fails, such as if the path does not exist in the bucket
    /// * `Could not sync` - If the sync fails
    pub async fn mv(&mut self, from_path_segments: Array, to_path_segments: Array) -> JsResult<()> {
        let from_path_segments = from_path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();
        let to_path_segments = to_path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/mv/{}/{} => {}",
            self.bucket.id.to_string(),
            &from_path_segments.join("/"),
            &to_path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        self.fs_metadata
            .as_mut()
            .unwrap()
            .mv(
                from_path_segments,
                to_path_segments,
                &self.metadata_blockstore,
            )
            .await
            .expect("could not mv");

        log!(
            "tomb-wasm: mount/mv/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    /// Rm a file or directory
    /// # Arguments
    /// * `path_segments` - The path to rm (as an Array)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not rm` - If the rm fails
    /// * `Could not sync` - If the sync fails
    pub async fn rm(&mut self, path_segments: Array) -> JsResult<()> {
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/rm/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        self.fs_metadata
            .as_mut()
            .unwrap()
            .rm(path_segments, &self.metadata_blockstore)
            .await
            .expect("could not rm");

        log!(
            "tomb-wasm: mount/rm/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    // TODO: migrate betwen mounts

    // TODO: Attaching approved keys to the metadata push
    /// Share with
    /// # Arguments
    /// * bucket_key_id - The id of the bucket key to share with
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `could not read bucket key` - If the bucket key cannot be read (such as if it does not exist, or does not belong to the bucket)
    /// * `Bucket is locked` - If the bucket is locked
    /// * `could not share with` - If the share fails
    #[wasm_bindgen(js_name = shareWith)]
    pub async fn share_with(&mut self, bucket_key_id: String) -> JsResult<()> {
        log!(
            "tomb-wasm: mount/share_with/{}/{}",
            self.bucket.id.to_string(),
            bucket_key_id.clone()
        );
        let bucket_id = self.bucket.id;
        let bucket_key_id = uuid::Uuid::parse_str(&bucket_key_id).expect("Invalid bucket_key UUID");

        let bucket_key = BucketKey::read(bucket_id, bucket_key_id, &mut self.client)
            .await
            .expect("could not read bucket key");

        let recipient_key = &bucket_key.pem;
        log!(
            "tomb-wasm: mount/share_with/{} - importing key",
            recipient_key.clone()
        );
        let recipient_key = &EcPublicEncryptionKey::import(recipient_key.as_bytes())
            .await
            .expect("could not import key");

        if self.locked() {
            panic!("Bucket is locked");
        };

        self.fs_metadata
            .as_mut()
            .unwrap()
            .share_with(recipient_key, &self.metadata_blockstore)
            .await
            .expect("could not share with");

        log!(
            "tomb-wasm: mount/share_with/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );

        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    /// Snapshot a mounted bucket
    /// # Returns
    /// A Promise<void> in js speak
    /// # Errors
    /// * "missing metadata" - If the metadata is missing
    /// * "could not snapshot" - If the snapshot fails
    #[wasm_bindgen(js_name = snapshot)]
    pub async fn snapshot(&mut self) -> JsResult<()> {
        log!("tomb-wasm: mount/snapshot/{}", self.bucket.id.to_string());
        // Get the bucket
        let metadata = self.metadata.as_ref();
        metadata
            .expect("missing metadata")
            .snapshot(&mut self.client)
            .await
            .expect("could not snapshot");
        // Ok
        Ok(())
    }

    /// Restore a mounted bucket
    /// # Arguments
    /// * `wasm_snapshot` - The snapshot to restore from
    /// # Returns
    /// A Promise<void> in js speak. Should update the mount to the version of the snapshot
    pub async fn restore(&mut self, wasm_snapshot: WasmSnapshot) -> JsResult<()> {
        log!(
            "tomb-wasm: mount/restore/{}/{}",
            self.bucket.id.to_string(),
            wasm_snapshot.id()
        );
        let snapshot = Snapshot::from(wasm_snapshot);
        snapshot
            .restore(&mut self.client)
            .await
            .expect("could not restore snapshot");

        Ok(())
    }
}
