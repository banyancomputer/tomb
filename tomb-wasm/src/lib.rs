//! This crate contains modules which are compiled to WASM

/// Expose Errors
mod error;

/// Mount implementation
pub mod mount;

/// Banyan API
pub mod types;

/// Misc utilities
pub mod utils;

use std::convert::From;
use std::convert::TryFrom;
use std::str::FromStr;

use gloo::console::log;
use js_sys::Array;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::{CryptoKey, CryptoKeyPair};

use tomb_common::banyan_api::client::{Client, Credentials};
use tomb_common::banyan_api::models::account::Account;
use tomb_common::banyan_api::models::{
    bucket::{Bucket, BucketType, StorageClass},
    bucket_key::*,
};
use tomb_crypt::prelude::*;

use crate::error::TombWasmError;
use crate::mount::WasmMount;
use crate::types::*;
use crate::utils::*;

pub type JsResult<T> = Result<T, js_sys::Error>;

#[wasm_bindgen]
pub struct TombWasm(pub(crate) Client);

/// TombWasm exposes the functionality of Tomb in a WASM module
#[wasm_bindgen]
impl TombWasm {
    fn client(&mut self) -> &mut Client {
        &mut self.0
    }

    // Note: Have to include this here so we can read the API key from the JS CryptoKey
    #[wasm_bindgen(constructor)]
    /// Create a new TombWasm instance
    /// # Arguments
    ///
    /// * `web_signing_key` - The CryptoKeyPair to use for signing requests
    /// * `account_id` - The id of the account to use
    /// * `api_endpoint` - The API endpoint to use
    ///
    /// # Returns
    ///
    /// A new TombWasm instance
    ///
    /// Don't call it from multiple threads in parallel!
    pub fn new(web_signing_key: CryptoKeyPair, account_id: String, api_endpoint: String) -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        set_panic_hook();

        // todo: this method needs to return a Result type but that would currently change the
        // external API so I'm leaving this one alone for now

        log!("tomb-wasm: new()");

        let mut banyan_client = Client::new(&api_endpoint).unwrap();

        let signing_key = EcSignatureKey::from(web_signing_key);
        let account_id = Uuid::parse_str(&account_id).unwrap();
        let banyan_credentials = Credentials {
            account_id,
            signing_key,
        };
        banyan_client.with_credentials(banyan_credentials);

        Self(banyan_client)
    }
}

impl From<Client> for TombWasm {
    fn from(client: Client) -> Self {
        Self(client)
    }
}

#[wasm_bindgen]
impl TombWasm {
    /*
     * Top level API Interface
     */

    /// Get the total consume storage space for the current account in bytes
    #[wasm_bindgen(js_name = getUsage)]
    pub async fn get_usage(&mut self) -> JsResult<u64> {
        Account::usage(self.client())
            .await
            .map_err(|err| TombWasmError(format!("failed to retrieve usage: {err}")).into())
    }

    /// Get the current usage limit for the current account in bytes
    #[wasm_bindgen(js_name = getUsageLimit)]
    pub async fn get_usage_limit(&mut self) -> JsResult<u64> {
        Account::usage_limit(self.client())
            .await
            .map_err(|err| TombWasmError(format!("failed to get usage limit: {err}")).into())
    }

    /// List the buckets for the current account
    #[wasm_bindgen(js_name = listBuckets)]
    pub async fn list_buckets(&mut self) -> JsResult<Array> {
        let buckets = Bucket::read_all(self.client())
            .await
            .map_err(|err| TombWasmError(format!("failed to read all buckets: {err}")))?;

        buckets
            .iter()
            .map(|bucket| {
                let wasm_bucket = WasmBucket::from(bucket.clone());
                JsValue::try_from(wasm_bucket).map_err(|err| {
                    TombWasmError(format!("failed to convert bucket to JsValue: {err}")).into()
                })
            })
            .collect()
    }

    /// List bucket snapshots for a bucket
    ///
    /// # Arguments
    ///
    /// * `bucket_id` - The id of the bucket to list snapshots for
    ///
    /// # Returns an array WasmSnapshots
    ///
    /// ```json
    /// [
    ///   {
    ///     "id": "ffc1dca2-5155-40be-adc6-c81eb7322fb8",
    ///     "bucket_id": "f0c55cc7-4896-4ff3-95de-76422af271b2",
    ///     "metadata_id": "05d063f1-1e3f-4876-8b16-aeb106af0eb0",
    ///     "created_at": "2023-09-05T19:05:34Z"
    ///   }
    /// ]
    /// ```
    #[wasm_bindgen(js_name = listBucketSnapshots)]
    pub async fn list_bucket_snapshots(&mut self, bucket_id: String) -> JsResult<Array> {
        log!("tomb-wasm: list_bucket_snapshots()");
        // Parse the bucket id
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();

        // Call the API
        let snapshots = Bucket::list_snapshots_by_bucket_id(self.client(), bucket_id)
            .await
            .map_err(|err| TombWasmError(format!("unable to list snapshots for bucket: {err}")))?;

        // Convert the snapshots
        snapshots
            .into_iter()
            .map(|snapshot| {
                let wasm_snapshot = WasmSnapshot::new(snapshot);
                JsValue::try_from(wasm_snapshot).map_err(|err| {
                    TombWasmError(format!("failed to convert snapshot to JsValue: {err}")).into()
                })
            })
            .collect()
    }

    /// List bucket keys for a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to list keys for
    /// # Returns an array of WasmBucketKeys in the form:
    /// ```json
    /// [
    /// {
    /// "id": "uuid",
    /// "bucket_id": "uuid",
    /// "pem": "string"
    /// "approved": "bool"
    /// }
    /// ]
    /// ```
    #[wasm_bindgen(js_name = listBucketKeys)]
    pub async fn list_bucket_keys(&mut self, bucket_id: String) -> JsResult<Array> {
        log!("tomb-wasm: list_bucket_keys()");
        // Parse the bucket id
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();

        // Call the API
        let keys = BucketKey::read_all(bucket_id, self.client())
            .await
            .map_err(|err| TombWasmError(format!("unable to read bucket keys: {err}")))?;

        // Convert the keys
        keys.iter()
            .map(|key| {
                let wasm_key = WasmBucketKey(key.clone());
                JsValue::try_from(wasm_key).map_err(|err| {
                    TombWasmError(format!("failed to convert key to JsValue: {err}")).into()
                })
            })
            .collect()
    }

    /// Create a new bucket
    /// # Arguments
    /// * `name` - The name of the bucket to create
    /// * `storage_class` - The storage class of the bucket to create
    /// * `bucket_type` - The type of the bucket to create
    /// * `encryption_key` - The encryption key to use for the bucket
    /// # Returns
    /// The bucket's metadata as a WasmBucket
    /// ```json
    /// {
    /// "id": "uuid",
    /// "name": "string"
    /// "type": "string",
    /// "storage_class": "string",
    /// }
    /// ```
    #[wasm_bindgen(js_name = createBucket)]
    pub async fn create_bucket(
        &mut self,
        name: String,
        storage_class: String,
        bucket_type: String,
        initial_key: CryptoKey,
    ) -> JsResult<WasmBucket> {
        log!("tomb-wasm: create_bucket()");
        let storage_class = StorageClass::from_str(&storage_class).expect("Invalid storage class");
        let bucket_type = BucketType::from_str(&bucket_type).expect("Invalid bucket type");
        let key = EcPublicEncryptionKey::from(initial_key);
        let pem_bytes = key.export().await.expect("Failed to export wrapping key");
        let pem = String::from_utf8(pem_bytes).expect("Failed to encode pem");
        // Call the API
        let (bucket, _bucket_key) =
            Bucket::create(name, pem, bucket_type, storage_class, self.client())
                .await
                .expect("Failed to create bucket");
        // Convert the bucket
        let wasm_bucket = WasmBucket::from(bucket);
        // Ok
        Ok(wasm_bucket)
    }

    /// Create a bucket key for a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to create a key for
    /// # Returns
    /// The WasmBucketKey that was created
    #[wasm_bindgen(js_name = createBucketKey)]
    pub async fn create_bucket_key(&mut self, bucket_id: String) -> JsResult<WasmBucketKey> {
        log!("tomb-wasm: create_bucket_key()");
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();

        // Load the EcEncryptionKey
        let key = EcEncryptionKey::generate()
            .await
            .map_err(|err| TombWasmError(format!("failed to generate ec encryption key: {err}")))?;
        let key = key.public_key().map_err(|err| {
            TombWasmError(format!(
                "unable to get public key from ec encryption key: {err}"
            ))
        })?;

        let pem = String::from_utf8(key.export().await.unwrap()).unwrap();

        // Call the API
        let bucket_key = BucketKey::create(bucket_id, pem, self.client())
            .await
            .map_err(|err| TombWasmError(format!("failed to create bucket: {err}")))?;

        Ok(WasmBucketKey(bucket_key))
    }

    /// Delete a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to delete
    /// # Returns the id of the bucket that was deleted
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket(&mut self, bucket_id: String) -> JsResult<()> {
        log!("tomb-wasm: delete_bucket()");

        // Parse the bucket id
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();

        // Call the API
        Bucket::delete_by_id(self.client(), bucket_id)
            .await
            .map_err(|err| TombWasmError(format!("failed to delete bucket: {err}")).into())
    }

    /* Bucket Mounting interface */

    /// Mount a bucket as a File System that can be managed by the user
    /// # Arguments
    /// * bucket_id - The id of the bucket to mount
    /// * key - The key to use to mount the bucket. This should be the crypto key pair that was used to create the bucket
    ///         or that has access to the bucket
    /// # Returns
    /// A WasmMount instance
    #[wasm_bindgen(js_name = mount)]
    pub async fn mount(&mut self, bucket_id: String, key: CryptoKeyPair) -> JsResult<WasmMount> {
        log!(format!("tomb-wasm: mount / {}", &bucket_id));

        // Parse the bucket id
        let bucket_id_uuid = Uuid::parse_str(&bucket_id).unwrap();
        log!(format!(
            "tomb-wasm: mount / {} / reading key pair",
            &bucket_id
        ));

        // Load the EcEncryptionKey
        let key = EcEncryptionKey::from(key);
        log!(format!(
            "tomb-wasm: mount / {} / reading bucket",
            &bucket_id
        ));

        // Load the bucket
        let bucket: WasmBucket = Bucket::read(self.client(), bucket_id_uuid)
            .await
            .map_err(|err| TombWasmError(format!("failed to read bucket: {err}")))?
            .into();

        log!(format!("tomb-wasm: mount / {} / pulling mount", &bucket_id));

        // Get the bucket id
        // Try to pull the mount. Otherwise create it and push an initial piece of metadata
        let mount = match WasmMount::pull(bucket.clone(), self.client()).await {
            Ok(mut mount) => {
                log!(format!(
                    "tomb-wasm: mount / {} / pulled mount, unlocking",
                    &bucket_id
                ));

                // Unlock the mount
                mount.unlock(&key).await?;
                log!(format!(
                    "tomb-wasm: mount / {} / unlocked mount",
                    &bucket_id
                ));

                // Ok
                mount
            }
            Err(_) => {
                log!(format!(
                    "tomb-wasm: mount / {} / failed to pull mount, creating",
                    &bucket_id
                ));
                // Create the mount and push an initial piece of metadata
                WasmMount::new(bucket.clone(), &key, self.client()).await?
            }
        };

        // Ok
        Ok(mount)
    }
}
