use async_trait::async_trait;
use walkdir::WalkDir;

use crate::utils::{is_visible, name_of};

use super::{OnDisk, OnDiskError};

// Extension of OnDisk for when I is String
#[async_trait(?Send)]
pub trait OnDiskExt<I>: OnDisk<I>
where
    I: std::fmt::Display,
{
    async fn id_from_string(value: String) -> Result<I, OnDiskError>;
    async fn decode_all() -> Result<Vec<Self>, OnDiskError> {
        let mut entries = Vec::new();
        for id in Self::entries()? {
            entries.push(Self::decode(&Self::id_from_string(id).await?).await?);
        }

        Ok(entries)
    }
}

/// Automatically implement that trait if its String
#[async_trait(?Send)]
impl<T> OnDiskExt<String> for T
where
    T: OnDisk<String>,
{
    async fn id_from_string(value: String) -> Result<String, OnDiskError> {
        Ok(value)
    }
}
