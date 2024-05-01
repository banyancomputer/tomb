/// this represents data that is stored locally on disk
pub mod local_share;
//mod xdg;

pub mod config;
use async_trait::async_trait;
use banyanfs::prelude::*;
use std::{
    fmt::Display,
    fs::{create_dir, File},
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Debug)]
pub enum DiskDataError {
    // Common error types we might find
    Var(std::env::VarError),
    Disk(std::io::Error),
    SerdeJson(serde_json::Error),
    //
    Implementation(String),
}

impl From<std::env::VarError> for DiskDataError {
    fn from(value: std::env::VarError) -> Self {
        Self::Var(value)
    }
}
impl From<std::io::Error> for DiskDataError {
    fn from(value: std::io::Error) -> Self {
        Self::Disk(value)
    }
}
impl From<serde_json::Error> for DiskDataError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}

impl Display for DiskDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}
impl std::error::Error for DiskDataError {}

pub enum DataType {
    Config,
    LocalShare,
}
impl DataType {
    pub fn root(&self) -> Result<PathBuf, DiskDataError> {
        let home = std::env::var("HOME")?;
        let path = match self {
            DataType::Config => PathBuf::from(format!("{home}/.local/share/banyan")),
            DataType::LocalShare => PathBuf::from(format!("{home}/.config/banyan")),
        };

        if !path.exists() {
            create_dir(&path)?;
        }

        Ok(path)
    }
}

#[async_trait(?Send)]
pub trait DiskData: Sized {
    const TYPE: DataType;
    const SUFFIX: &'static str;
    const EXTENSION: &'static str;

    fn path(identifier: &str) -> Result<PathBuf, DiskDataError> {
        Ok(Self::TYPE.root()?.join(Self::SUFFIX).join(format!(
            "{}.{}",
            identifier,
            Self::EXTENSION
        )))
    }
    async fn encode(&self, identifier: &str) -> Result<(), DiskDataError>;
    async fn decode(identifier: &str) -> Result<Self, DiskDataError>;
}
