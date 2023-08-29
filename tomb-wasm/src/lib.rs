//! This crate contains modules which are compiled to WASM
#![warn(rust_2018_idioms)]
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
use web_sys::CryptoKey;

use tomb_common::banyan_api::client::{Client, Credentials};
use tomb_common::banyan_api::models::account::Account;
use tomb_common::banyan_api::models::bucket::{Bucket, BucketType, StorageClass};
use tomb_crypt::prelude::*;
use web_sys::CryptoKeyPair;

use crate::error::TombWasmError;
use crate::mount::WasmMount;
use crate::types::WasmBucket;
use crate::utils::{set_panic_hook, JsResult};

// #[derive(Debug)]
#[wasm_bindgen]
pub struct TombWasm(pub(crate) Client);

#[wasm_bindgen]
/// TombWasm exposes the functionality of Tomb in a WASM module
impl TombWasm {
    // Note: Have to include this here so we can read the API key from the JS CryptoKey
    #[wasm_bindgen(constructor)]
    /// Create a new TombWasm instance
    pub fn new(web_signing_key: CryptoKeyPair, account_id: String, api_endpoint: String) -> Self {
        set_panic_hook();
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

    fn client(&mut self) -> &mut Client {
        &mut self.0
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

    /// Get the Total Usage for the current account
    /// # Returns
    /// The total storage used by the account, in bytes
    #[wasm_bindgen(js_name = getUsage)]
    pub async fn get_usage(&mut self) -> JsResult<u64> {
        log!("tomb-wasm: get_usage");
        let size = Account::usage(self.client())
            .await
            .expect("Failed to get usage");
        Ok(size)
    }

    /// Get the Usage limit for the current account
    /// # Returns
    /// The storage limit for the account in bytes (this should be 5 TiB)
    #[wasm_bindgen(js_name = getUsageLimit)]
    pub async fn get_usage_limit(&mut self) -> JsResult<u64> {
        log!("tomb-wasm: get_usage_limit");
        let size = Account::usage_limit(self.client())
            .await
            .expect("Failed to get usage limit");
        Ok(size)
    }

    /// List the buckets for the current account
    /// # Returns
    /// An array of buckets of the form:
    /// ```json
    /// [
    ///   {
    ///    "id": "uuid",
    ///    "name": "string"
    ///   "type": "string",
    ///  "storage_class": "string",
    ///   }
    /// ]
    /// ```
    pub async fn list_buckets(&mut self) -> JsResult<Array> {
        log!("tomb-wasm: list_buckets()");
        let buckets = Bucket::read_all(self.client())
            .await
            .map_err(|_| TombWasmError::unknown_error())?;
        // Iterate over the buckets and turn them into Wasm Buckets
        let buckets = buckets
            .iter()
            .map(|bucket| {
                let wasm_bucket = WasmBucket::from(bucket.clone());
                JsValue::try_from(wasm_bucket).expect("Failed to convert bucket to JsValue")
            })
            .collect::<Array>();
        // Ok
        Ok(buckets)
    }

    /// Create a new bucket
    /// # Arguments
    /// * `name` - The name of the bucket to create
    /// * `storage_class` - The storage class of the bucket to create
    /// * `bucket_type` - The type of the bucket to create
    /// * `encryption_key` - The encryption key to use for the bucket
    /// # Returns
    /// The bucket that was created
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
        let (bucket, bucket_key) =
            Bucket::create(name, pem, bucket_type, storage_class, self.client())
                .await
                .expect("Failed to create bucket");

        // Convert the bucket
        let wasm_bucket = WasmBucket::from(bucket);

        // Ok
        Ok(wasm_bucket)
    }

    /// Delete a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to delete
    /// # Returns the id of the bucket that was deleted
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket(&mut self, bucket_id: String) -> JsResult<String> {
        log!("tomb-wasm: delete_bucket()");
        // Parse the bucket id
        let bucket_id = Uuid::parse_str(&bucket_id).unwrap();
        // Call the API
        let response = Bucket::delete_by_id(self.client(), bucket_id)
            .await
            .map_err(|_| TombWasmError::unknown_error())?;
        Ok(response)
    }

    /* Bucket Mounting interface */

    /// Initialize a bucket by id. Returns a mount object
    /// # Arguments
    /// * bucket_id - The id of the bucket to mount
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
            .map_err(|_| TombWasmError::unknown_error())?
            .into();
        log!(format!("tomb-wasm: mount / {} / pulling mount", &bucket_id));
        // Get the bucket id
        // Try to pull the mount. Otherwise create it and push an initial piece of metadata
        let mount = match WasmMount::pull(bucket.clone(), self.client()).await {
            Ok(mount) => {
                log!(format!(
                    "tomb-wasm: mount / {} / pulled mount, unlocking",
                    &bucket_id
                ));
                // Unlock the mount
                mount.unlock(&key).await?
            }
            Err(_) => {
                log!(format!(
                    "tomb-wasm: mount / {} / failed to pull mount, creating",
                    &bucket_id
                ));
                // Create the mount and push an initial piece of metadata
                let mut mount = WasmMount::new(bucket.clone(), &key, self.client()).await?;
                // Ok
                mount
            }
        };
        // Ok
        Ok(mount)
    }
}
