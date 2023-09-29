use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::{
    client::Client,
    error::ClientError,
    requests::core::buckets::keys::{create::*, delete::*, read::*, reject::*},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
/// BucketKey Definition
pub struct BucketKey {
    /// The unique identifier for the Bucket Key
    pub id: Uuid,
    /// The unique identifier for the bucket it belongs to
    pub bucket_id: Uuid,
    /// The public key material for the Bucket Key
    pub pem: String,
    /// The public key fingerprint for the Bucket Key
    pub fingerprint: String,
    /// Whether or not the Bucket Key has been approved
    pub approved: bool,
}

impl Display for BucketKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.approved {
            "approved"
        } else {
            "unapproved"
        };
        f.write_fmt(format_args!(
            "\n| KEY INFO |\nbucket_id: {}\nfingerprint: {}\nstatus: {}",
            self.bucket_id, self.fingerprint, status
        ))
    }
}

impl BucketKey {
    /// Create a new Bucket Key
    pub async fn create(
        bucket_id: Uuid,
        pem: String,
        client: &mut Client,
    ) -> Result<Self, ClientError> {
        let response: CreateBucketKeyResponse = client
            .call_core(CreateBucketKey {
                bucket_id,
                pem: pem.clone(),
            })
            .await?;
        Ok(Self {
            id: response.id,
            bucket_id,
            pem,
            fingerprint: response.fingerprint,
            approved: response.approved,
        })
    }

    /// Read all Bucket Keys for a bucket
    pub async fn read_all(bucket_id: Uuid, client: &mut Client) -> Result<Vec<Self>, ClientError> {
        let response: ReadAllBucketKeysResponse =
            client.call_core(ReadAllBucketKeys { bucket_id }).await?;
        let mut bucket_keys = Vec::new();
        for key in response.0 {
            bucket_keys.push(Self {
                id: key.id,
                bucket_id,
                pem: key.pem,
                fingerprint: key.fingerprint,
                approved: key.approved,
            });
        }
        Ok(bucket_keys)
    }

    /// Read a Bucket Key
    pub async fn read(bucket_id: Uuid, id: Uuid, client: &mut Client) -> Result<Self, ClientError> {
        let response: ReadBucketKeyResponse = client.call_core(ReadBucketKey { bucket_id, id }).await?;
        Ok(Self {
            id: response.id,
            bucket_id,
            pem: response.pem,
            fingerprint: response.fingerprint,
            approved: response.approved,
        })
    }

    /// Delete a Bucket Key
    pub async fn delete(self, client: &mut Client) -> Result<String, ClientError> {
        let response = client
            .call_core(DeleteBucketKey {
                bucket_id: self.bucket_id,
                id: self.id,
            })
            .await?;
        Ok(response.id.to_string())
    }

    /// Delete a Bucket Key by id
    pub async fn delete_by_id(
        bucket_id: Uuid,
        id: Uuid,
        client: &mut Client,
    ) -> Result<String, ClientError> {
        let response = client.call_core(DeleteBucketKey { bucket_id, id }).await?;
        Ok(response.id.to_string())
    }

    /// Reject a Bucket Key
    pub async fn reject(
        bucket_id: Uuid,
        id: Uuid,
        client: &mut Client,
    ) -> Result<String, ClientError> {
        Ok(client
            .call_core(RejectBucketKey { bucket_id, id })
            .await?
            .id
            .to_string())
    }
}

#[cfg(test)]
mod test {
    use tomb_crypt::prelude::{EcPublicEncryptionKey, PrivateKey, PublicKey};
    use tomb_crypt::pretty_fingerprint;

    use super::*;
    use crate::banyan_api::models::account::test::authenticated_client;
    use crate::banyan_api::models::bucket::test::create_bucket;
    use crate::banyan_api::models::metadata::Metadata;
    use crate::banyan_api::utils::generate_bucket_key;

    #[tokio::test]
    async fn create() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem.clone(), &mut client).await?;
        assert_eq!(bucket_key.bucket_id, bucket.id);
        assert!(!bucket_key.approved);
        assert_eq!(bucket_key.pem, pem);
        Ok(())
    }

    #[tokio::test]
    async fn create_read() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (key, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let our_fingerprint =
            pretty_fingerprint(key.fingerprint().await.expect("cant fingerprint").as_slice());
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        assert_eq!(our_fingerprint, bucket_key.fingerprint);
        let read_bucket_key = BucketKey::read(bucket.id, bucket_key.id, &mut client).await?;
        assert_eq!(bucket_key.id, read_bucket_key.id);
        assert_eq!(bucket_key.bucket_id, read_bucket_key.bucket_id);
        assert_eq!(bucket_key.approved, read_bucket_key.approved);
        assert_eq!(bucket_key.pem, read_bucket_key.pem);
        assert_eq!(bucket_key.fingerprint, read_bucket_key.fingerprint);
        Ok(())
    }

    #[tokio::test]
    async fn create_read_all() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        let bucket_keys = BucketKey::read_all(bucket.id, &mut client).await?;
        assert_eq!(bucket_keys.len(), 2);
        assert_eq!(bucket_key.id, bucket_keys[1].id);
        assert_eq!(bucket_key.bucket_id, bucket_keys[1].bucket_id);
        assert_eq!(bucket_key.approved, bucket_keys[1].approved);
        assert_eq!(bucket_key.pem, bucket_keys[1].pem);
        Ok(())
    }

    #[tokio::test]
    async fn create_delete() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        let bucket_key_ = bucket_key.clone();
        let id = bucket_key.delete(&mut client).await?;
        assert_eq!(id, bucket_key_.id.to_string());
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn create_delete_by_id_for_extant_bucket() {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await.unwrap();
        let fake_bucket_key_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let _ = BucketKey::delete_by_id(bucket.id, fake_bucket_key_id, &mut client)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn create_delete_by_id_for_non_extant_bucket() {
        let mut client = authenticated_client().await;
        let fake_bucket_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let fake_bucket_key_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let _ = BucketKey::delete_by_id(fake_bucket_id, fake_bucket_key_id, &mut client)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_reject() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_bucket_key().await;
        let (bucket, _) = create_bucket(&mut client).await?;
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        assert!(!bucket_key.approved);
        let rejected_id = BucketKey::reject(bucket.id, bucket_key.id, &mut client).await?;
        assert_eq!(bucket_key.id.to_string(), rejected_id);
        Ok(())
    }

    #[tokio::test]
    async fn reject_approved_key() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, initial_bucket_key) = create_bucket(&mut client).await?;
        assert!(initial_bucket_key.approved);
        let rejected_id = BucketKey::reject(bucket.id, initial_bucket_key.id, &mut client).await;
        assert!(rejected_id.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn approve_new_key() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, initial_bucket_key) = create_bucket(&mut client).await?;
        // Create a new bucket key
        let (_, pem) = generate_bucket_key().await;
        let bucket_key = BucketKey::create(bucket.id, pem, &mut client).await?;
        assert!(!bucket_key.approved);

        let fingerprint1 = pretty_fingerprint(
            EcPublicEncryptionKey::import(initial_bucket_key.pem.as_bytes())
                .await
                .unwrap()
                .fingerprint()
                .await
                .unwrap()
                .as_slice(),
        );
        let fingerprint2 = pretty_fingerprint(
            EcPublicEncryptionKey::import(bucket_key.pem.as_bytes())
                .await
                .unwrap()
                .fingerprint()
                .await
                .unwrap()
                .as_slice(),
        );

        // Push metadata with the new BucketKey listed as valid
        Metadata::push(
            bucket.id,
            "root_cid".to_string(),
            "metadata_cid".to_string(),
            0,
            vec![fingerprint1, fingerprint2],
            "metadata_stream".as_bytes(),
            &mut client,
        )
        .await?;

        // Read the bucket key again
        let updated_bucket_key = BucketKey::read(bucket.id, bucket_key.id, &mut client).await?;
        assert!(updated_bucket_key.approved);

        Ok(())
    }
}
