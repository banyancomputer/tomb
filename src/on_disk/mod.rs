//! this represents data that is stored locally on disk

mod error;
pub use error::*;
pub mod config;
mod ext;
pub mod local_share;
use async_trait::async_trait;
pub use ext::*;
use std::{
    fmt::Display,
    fs::{create_dir, remove_file},
    path::PathBuf,
};
use tokio::fs::{remove_dir_all, rename, File, OpenOptions};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use walkdir::WalkDir;

use crate::utils::{is_visible, name_of};
pub enum DiskType {
    Config,
    LocalShare,
}

impl DiskType {
    pub fn root(&self) -> PathBuf {
        let home = env!("HOME");
        match self {
            DiskType::Config => PathBuf::from(format!("{home}/.config/banyan")),
            DiskType::LocalShare => PathBuf::from(format!("{home}/.local/share/banyan")),
        }
    }

    pub fn init(&self) -> Result<(), OnDiskError> {
        if !self.root().exists() {
            create_dir(self.root())?;
        }
        Ok(())
    }
}

/// The purpose of this trait is to standardize file encoding and decoding implementations
/// for files that need to live in XDG home.
#[async_trait(?Send)]
pub trait OnDisk<I: Display>: Sized {
    const TYPE: DiskType;
    const SUFFIX: &'static str;
    const EXTENSION: &'static str;

    fn container() -> PathBuf {
        Self::TYPE.root().join(Self::SUFFIX)
    }

    fn exists(identifier: &I) -> Result<bool, OnDiskError> {
        Ok(Self::path(identifier)?.exists())
    }

    fn path(identifier: &I) -> Result<PathBuf, OnDiskError> {
        let container = Self::container();
        if !container.exists() {
            create_dir(&container)?;
        }
        // Folder path
        if Self::EXTENSION.is_empty() {
            Ok(container.join(identifier.to_string()))
        }
        // File path
        else {
            Ok(container.join(format!("{}.{}", identifier, Self::EXTENSION)))
        }
    }

    async fn erase(identifier: &I) -> Result<(), OnDiskError> {
        // Folder path
        if Self::EXTENSION.is_empty() {
            Ok(remove_dir_all(Self::path(identifier)?).await?)
        }
        // File path
        else {
            Ok(remove_file(Self::path(identifier)?)?)
        }
    }

    async fn rename(old: &I, new: &I) -> Result<(), OnDiskError> {
        let old_path = Self::path(old)?;
        let new_path = Self::path(new)?;
        rename(old_path, new_path).await?;
        Ok(())
    }

    fn entries() -> Vec<String> {
        let container = Self::container();
        if !container.exists() {
            vec![]
        } else {
            WalkDir::new(container)
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
                // Strip file extensions
                .map(|id| {
                    id.trim_end_matches(&format!(".{}", Self::EXTENSION))
                        .to_string()
                })
                .collect()
        }
    }

    // Async compat reader/writer defaults
    async fn get_writer(identifier: &I) -> Result<Compat<File>, OnDiskError> {
        let mut file_opts = OpenOptions::new();
        file_opts.write(true);
        file_opts.create(true);
        file_opts.truncate(true);
        Ok(file_opts.open(Self::path(identifier)?).await?.compat())
    }
    async fn get_reader(identifier: &I) -> Result<Compat<File>, OnDiskError> {
        Ok(File::open(Self::path(identifier)?).await?.compat())
    }

    async fn encode(&self, identifier: &I) -> Result<(), OnDiskError>;
    async fn decode(identifier: &I) -> Result<Self, OnDiskError>;
}
