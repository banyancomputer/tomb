//! this represents data that is stored locally on disk

pub mod config;
pub mod local_share;
use async_trait::async_trait;
use std::{fmt::Display, fs::create_dir, path::PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use walkdir::WalkDir;

use crate::utils::{is_visible, name_of};
#[derive(Debug)]
pub enum OnDiskError {
    // Common error types we might find
    Disk(std::io::Error),
    SerdeJson(serde_json::Error),
    //
    Implementation(String),
}

impl From<std::io::Error> for OnDiskError {
    fn from(value: std::io::Error) -> Self {
        Self::Disk(value)
    }
}
impl From<serde_json::Error> for OnDiskError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}

impl Display for OnDiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}
impl std::error::Error for OnDiskError {}

pub enum DiskType {
    Config,
    LocalShare,
}
impl DiskType {
    pub fn root(&self) -> Result<PathBuf, OnDiskError> {
        let home = env!("HOME");
        let path = match self {
            DiskType::Config => PathBuf::from(format!("{home}/.local/share/banyan")),
            DiskType::LocalShare => PathBuf::from(format!("{home}/.config/banyan")),
        };

        if !path.exists() {
            create_dir(&path)?;
        }

        Ok(path)
    }
}

/// The purpose of this trait is to standardize file encoding and decoding implementations
/// for files that need to live in XDG home.
#[async_trait(?Send)]
pub trait OnDisk<I: Display>: Sized {
    const TYPE: DiskType;
    const SUFFIX: &'static str;
    const EXTENSION: &'static str;

    fn container() -> Result<PathBuf, OnDiskError> {
        Ok(Self::TYPE.root()?.join(Self::SUFFIX))
    }

    fn path(identifier: &I) -> Result<PathBuf, OnDiskError> {
        Ok(Self::container()?.join(format!("{}.{}", identifier, Self::EXTENSION)))
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
/*
fn get_all() -> Result<Vec<Self>, OnDiskError> {
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
        .map(|id| Self::decode(id))
    {}

    todo!()
}
*/
