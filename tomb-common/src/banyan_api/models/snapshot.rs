use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

use crate::banyan_api::{
    client::Client, error::ClientError, models::metadata::Metadata,
    requests::core::buckets::snapshots::restore::*,
};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
/// Bucket Snapshot Definition
pub struct Snapshot {
    /// The unique identifier for the bucket metadata
    pub id: Uuid,
    /// The unique identifier for the bucket it belongs to
    pub bucket_id: Uuid,
    /// The unique identifier for the bucket it belongs to
    pub metadata_id: Uuid,
    /// The size of the data associated with the snapshot
    pub size: u64,
    /// The timestamp when the snapshot was created
    pub created_at: i64,
}

impl Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "\n| SNAPSHOT INFO |\nsnapshot_id:\t{}\nbucket_id:\t{}\nmetadata_id:\t{}\ncreated_at:\t{}",
            self.id,
            self.bucket_id,
            self.metadata_id,
            self.created_at
        ))
    }
}

impl Snapshot {
    /// Restore a snapshot to its bucket
    pub async fn restore(&self, client: &mut Client) -> Result<Uuid, ClientError> {
        let request = RestoreSnapshot {
            bucket_id: self.bucket_id,
            snapshot_id: self.id,
        };
        let response = client.call(request).await?;
        Ok(response.metadata_id)
    }
    /// Get the metadata for this snapshot
    pub async fn metadata(&self, client: &mut Client) -> Result<Metadata, ClientError> {
        Metadata::read(self.bucket_id, self.metadata_id, client).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::banyan_api::models::account::test::authenticated_client;
    use crate::banyan_api::models::bucket::test::create_bucket;
    use crate::banyan_api::models::metadata::test::push_empty_metadata;
    use crate::banyan_api::models::metadata::{Metadata, MetadataState};

    #[tokio::test]
    async fn restore() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (bucket, _) = create_bucket(&mut client).await.unwrap();
        let (metadata, _) = push_empty_metadata(bucket.id, &mut client).await.unwrap();
        let snapshot = metadata.snapshot(&mut client).await.unwrap();
        let restored_metadata_id = snapshot.restore(&mut client).await.unwrap();
        assert_eq!(restored_metadata_id, metadata.id);
        let restored_metadata = Metadata::read(bucket.id, restored_metadata_id, &mut client)
            .await
            .unwrap();
        assert_eq!(restored_metadata.id, metadata.id);
        assert_eq!(metadata.bucket_id, restored_metadata.bucket_id);
        assert_eq!(restored_metadata.state, MetadataState::Current);
        Ok(())
    }
}
