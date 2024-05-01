//! this represents data that is stored locally on disk

pub mod config;
pub mod local_share;
use async_trait::async_trait;
use std::{fmt::Display, fs::create_dir, path::PathBuf};

#[derive(Debug)]
pub enum DiskDataError {
    // Common error types we might find
    Disk(std::io::Error),
    SerdeJson(serde_json::Error),
    //
    Implementation(String),
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
        let home = env!("HOME");
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
pub trait DiskData<I: Display>: Sized {
    const TYPE: DataType;
    const SUFFIX: &'static str;
    const EXTENSION: &'static str;

    fn path(identifier: &I) -> Result<PathBuf, DiskDataError> {
        Ok(Self::TYPE.root()?.join(Self::SUFFIX).join(format!(
            "{}.{}",
            identifier,
            Self::EXTENSION
        )))
    }
    async fn encode(&self, identifier: &I) -> Result<(), DiskDataError>;
    async fn decode(identifier: &I) -> Result<Self, DiskDataError>;
}
