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
        for id in WalkDir::new(Self::container()?)
            // Should never go deep
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            // File is visible
            .filter_entry(is_visible)
            // User has permission
            .filter_map(|e| e.ok())
            // Turn into ids
            .filter_map(|e| name_of(e.path()))
        {
            entries.push(Self::decode(&Self::id_from_string(id).await?).await?);
        }

        Ok(entries)
    }
}

/*
#[async_trait(?Send)]
impl<T> OnDiskExt for T
where
    T: OnDisk<String>,
{
    async fn decode_all() -> Result<Vec<Self>, OnDiskError> {
    }
}
*/
