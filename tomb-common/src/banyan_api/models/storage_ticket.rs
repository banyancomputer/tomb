use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
#[cfg(target_arch = "wasm32")]
use std::io::Read;

use crate::banyan_api::{
    client::Client, error::ClientError, requests::staging::client_grant::create::*,
};
use tomb_crypt::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// StorageTicket is a ticket that can be used authenticate requests to stage data to a storage host
pub struct StorageTicket {
    /// The host to stage data to
    pub host: String,
    /// The authorization token to use when staging data. Generated by the core service
    pub authorization: String,
}

impl Display for StorageTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "\n{}\nhost:\t{}\nauthorization:\t{}",
            "| STORAGE TICKET INFO |".yellow(),
            self.host,
            self.authorization
        ))
    }
}

impl StorageTicket {
    /// Create a new grant for a client to stage data to a storage host
    /// Allows us to upload data to a storage host using our signing key
    pub async fn create_grant(self, client: &mut Client) -> Result<(), ClientError> {
        let signing_key = client
            .signing_key
            .as_ref()
            .expect("Client signing key not set");
        let public_key_bytes = signing_key
            .public_key()
            .expect("Failed to get public key")
            .export()
            .await
            .expect("Failed to export public key");
        let public_key =
            String::from_utf8(public_key_bytes).expect("Failed to convert public key to string");
        client
            .call_no_content(CreateGrant {
                host_url: self.host.clone(),
                bearer_token: self.authorization.clone(),
                public_key,
            })
            .await
    }

    // // TODO: This should probably take a generic trait related to Tomb in order to restore these arguments
    // /// Push new Metadata for a bucket. Creates a new metadata records and returns a storage ticket
    // #[cfg(not(target_arch = "wasm32"))]
    // pub async fn upload_content<S>(
    //     self,
    //     // TODO: This should probably be a metadata cid
    //     metadata_id: Uuid,
    //     content: S,
    //     content_len: u64,
    //     content_hash: String,
    //     client: &mut Client,
    // ) -> Result<(), ClientError>
    // where
    //     reqwest::Body: From<S>,
    // {
    //     client
    //         .multipart_no_content(PushContent {
    //             host_url: self.host.clone(),
    //             metadata_id,
    //             content,
    //             content_len,
    //             content_hash,
    //         })
    //         .await
    // }

    // #[cfg(target_arch = "wasm32")]
    // /// Push new metadata for a bucket. Creates a new metadata record and returns a storage ticket if needed
    // /// WASM implementation because reqwest hates me
    // pub async fn upload_content<S>(
    //     self,
    //     metadata_id: Uuid,
    //     content: S,
    //     content_len: u64,
    //     content_hash: String,
    //     client: &mut Client,
    // ) -> Result<(), ClientError>
    // where
    //     S: Read,
    // {
    //     client
    //         .multipart_no_content(PushContent {
    //             host_url: self.host.clone(),
    //             metadata_id,
    //             content,
    //             content_len,
    //             content_hash,
    //         })
    //         .await
    // }
}

#[cfg(test)]
#[cfg(feature = "fake")]
pub mod test {
    use std::collections::BTreeSet;
    use tomb_crypt::hex_fingerprint;
    use wnfs::libipld::Cid;

    use super::*;
    use crate::banyan_api::blockstore::BanyanApiBlockStore;
    use crate::banyan_api::models::account::test::authenticated_client;
    use crate::banyan_api::models::account::Account;
    use crate::banyan_api::models::bucket::{Bucket, BucketType, StorageClass};
    use crate::banyan_api::models::bucket_key::BucketKey;
    use crate::banyan_api::models::metadata::Metadata;
    use crate::banyan_api::utils::generate_bucket_key;
    use crate::blockstore::carv2_memory::CarV2MemoryBlockStore;
    use crate::blockstore::RootedBlockStore;
    use crate::metadata::FsMetadata;

    #[tokio::test]
    async fn authorization_grants() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (
            bucket,
            _bucket_key,
            _key,
            metadata,
            storage_ticket,
            _metadata_store,
            content_store,
            _fs_metadata,
            _add_path_segments,
        ) = setup(&mut client).await?;

        // Create a grant using storage ticket
        storage_ticket.clone().create_grant(&mut client).await?;

        // Assert 404 before any space has been allocated
        assert!(bucket.get_grants_token(&mut client).await.is_err());

        content_store
            .upload(Some(storage_ticket.host), metadata.id, &mut client)
            .await?;

        let account = Account::who_am_i(&mut client).await.unwrap();
        println!("bucket_id: {}, account_id: {}", bucket.id, account.id);

        // Successfully get a new client with a bearer token which can access the new grants
        let _new_client = bucket.get_grants_token(&mut client).await?;

        Ok(())
    }

    #[tokio::test]
    async fn create_grant() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (
            _bucket,
            _bucket_key,
            _key,
            metadata,
            storage_ticket,
            metadata_store,
            content_store,
            mut fs_metadata,
            add_path_segments,
        ) = setup(&mut client).await?;
        storage_ticket.clone().create_grant(&mut client).await?;
        content_store
            .upload(Some(storage_ticket.host.clone()), metadata.id, &mut client)
            .await?;
        let mut blockstore_client = client.clone();
        blockstore_client
            .with_remote(&storage_ticket.host)
            .expect("Failed to create blockstore client");
        let banyan_api_blockstore = BanyanApiBlockStore::from(blockstore_client);
        let bytes = fs_metadata
            .read(&add_path_segments, &metadata_store, &banyan_api_blockstore)
            .await
            .expect("Failed to get file");
        assert_eq!(bytes, "test".as_bytes().to_vec());
        Ok(())
    }

    #[tokio::test]
    async fn get_locations() -> Result<(), ClientError> {
        use crate::banyan_api::requests::core::blocks::locate::LocationRequest;
        let mut client = authenticated_client().await;
        let (
            _bucket,
            _bucket_key,
            _key,
            metadata,
            storage_ticket,
            metadata_store,
            content_store,
            mut fs_metadata,
            add_path_segments,
        ) = setup(&mut client).await?;
        storage_ticket.clone().create_grant(&mut client).await?;
        content_store
            .upload(Some(storage_ticket.host.clone()), metadata.id, &mut client)
            .await?;
        let mut blockstore_client = client.clone();
        blockstore_client
            .with_remote(&storage_ticket.host)
            .expect("Failed to create blockstore client");
        let banyan_api_blockstore = BanyanApiBlockStore::from(blockstore_client);
        let bytes = fs_metadata
            .read(&add_path_segments, &metadata_store, &banyan_api_blockstore)
            .await
            .expect("Failed to get file");
        assert_eq!(bytes, "test".as_bytes().to_vec());

        let cids: Vec<Cid> = content_store.car.car.index.borrow().get_all_cids();
        let cids_request: LocationRequest = cids
            .clone()
            .into_iter()
            .map(|cid| cid.to_string())
            .collect();
        let locations = client
            .call(cids_request)
            .await
            .expect("Failed to get locations");

        let stored_blocks = locations
            .get(&storage_ticket.host)
            .expect("no blocks at storage host");
        for cid in cids {
            assert!(stored_blocks.contains(&cid.to_string()));
        }
        Ok(())
    }

    #[tokio::test]
    async fn get_bad_location() -> Result<(), ClientError> {
        use crate::banyan_api::requests::core::blocks::locate::LocationRequest;
        let mut client = authenticated_client().await;
        let cids: LocationRequest = vec![cid::Cid::default().to_string()];
        let locations = client
            .call(cids.clone())
            .await
            .expect("Failed to get locations");
        let target_cids = locations.get("NA").expect("Failed to get cids");
        for cid in cids.clone() {
            assert!(target_cids.contains(&cid));
        }
        Ok(())
    }

    async fn create_bucket_v2(
        client: &mut Client,
    ) -> Result<(Bucket, BucketKey, EcEncryptionKey), ClientError> {
        let (key, pem) = generate_bucket_key().await;
        let bucket_type = BucketType::Interactive;
        let bucket_class = StorageClass::Hot;
        let bucket_name = format!("{}", rand::random::<u64>());
        let fingerprint = hex_fingerprint(
            key.fingerprint()
                .await
                .expect("create fingerprint")
                .as_slice(),
        );
        let (bucket, bucket_key) = Bucket::create(
            bucket_name.clone(),
            pem.clone(),
            bucket_type,
            bucket_class,
            client,
        )
        .await?;
        assert_eq!(bucket.name, bucket_name.clone());
        assert_eq!(bucket.r#type, bucket_type.clone());
        assert!(bucket_key.approved);
        assert_eq!(bucket_key.pem, pem);
        assert_eq!(bucket_key.fingerprint, fingerprint);
        assert!(bucket_key.approved);
        Ok((bucket, bucket_key, key))
    }

    async fn setup(
        client: &mut Client,
    ) -> Result<
        (
            Bucket,
            BucketKey,
            EcEncryptionKey,
            Metadata,
            StorageTicket,
            CarV2MemoryBlockStore,
            CarV2MemoryBlockStore,
            FsMetadata,
            Vec<String>,
        ),
        ClientError,
    > {
        let (bucket, bucket_key, key) = create_bucket_v2(client).await?;
        let metadata_store = CarV2MemoryBlockStore::new().expect("Failed to create metadata store");
        let content_store = CarV2MemoryBlockStore::new().expect("Failed to create content store");
        let mut fs_metadata = FsMetadata::init(&key)
            .await
            .expect("Failed to create fs metadata");
        let mkdir_path_segments = vec!["test".to_string(), "path".to_string()];
        let add_path_segments = vec!["test".to_string(), "path".to_string(), "file".to_string()];
        let file_content = "test".as_bytes().to_vec();
        fs_metadata
            .mkdir(&mkdir_path_segments, &metadata_store)
            .await
            .expect("Failed to create directory");
        fs_metadata
            .write(
                &add_path_segments,
                &metadata_store,
                &content_store,
                file_content,
            )
            .await
            .expect("Failed to add file");
        fs_metadata
            .save(&metadata_store, &content_store)
            .await
            .expect("Failed to save fs metadata");
        let root_cid = &content_store.get_root().expect("Failed to get root cid");
        let metadata_cid = &metadata_store
            .get_root()
            .expect("Failed to get metadata cid");
        let data_size = content_store.data_size();
        let metadata_bytes = metadata_store.get_data();
        let (metadata, host, authorization) = Metadata::push(
            bucket.id,
            root_cid.to_string(),
            metadata_cid.to_string(),
            data_size,
            vec![],
            BTreeSet::new(),
            metadata_bytes,
            client,
        )
        .await?;
        let storage_ticket = StorageTicket {
            host: host.unwrap(),
            authorization: authorization.unwrap(),
        };
        Ok((
            bucket,
            bucket_key,
            key,
            metadata,
            storage_ticket,
            metadata_store,
            content_store,
            fs_metadata,
            add_path_segments,
        ))
    }
}
