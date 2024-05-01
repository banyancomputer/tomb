use crate::NativeError;
use async_trait::async_trait;
use banyanfs::prelude::*;
use std::{
    fmt::Display,
    fs::{create_dir, File},
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Debug)]
enum DiskDataError {
    Disk(std::io::Error),
}

impl Display for DiskDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}
impl std::error::Error for DiskDataError {}

impl From<std::io::Error> for DiskDataError {
    fn from(value: std::io::Error) -> Self {
        Self::Disk(value)
    }
}

enum DataType {
    Config,
    LocalShare
}
impl DataType {
    fn root(&self) -> PathBuf {
        let home = std::env::var("HOME").expect("Set the $HOME env var");
        let path = match self {
            DataType::Config => PathBuf::from(format!("{home}/.local/share/banyan")),
            DataType::LocalShare => PathBuf::from(format!("{home}/.config/banyan")),
        };

        if !path.exists() {
            create_dir(path).expect("Creating dir failed");
        }

        path
    }
}


#[async_trait(?Send)]
pub trait DiskData: Sized {
    const TYPE: DataType;
    const SUFFIX: String;
    const EXTENSION: String;

    fn path(identifier: String) -> PathBuf {
        Self::TYPE.root().join(Self::SUFFIX).join(format!("{}.{}", identifier, Self::EXTENSION))
    }
    async fn encode(identifier: String, value: Self) -> Result<(), DiskDataError>;
    async fn decode(identifier: String) -> Result<Self, DiskDataError>;
}

#[async_trait(?Send)]
impl DiskData for SigningKey {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: String = String::from("user_keys");
    const EXTENSION: String = String::from("pem");
        
    async fn encode(identifier: String, value: Self) -> Result<(), DiskDataError> {
        let mut writer = File::create(Self::path(identifier))?;
        let pem: String = value.to_pkcs8_pem().unwrap().to_string();
        writer.write_all(pem.as_bytes())?;
        Ok(())
    }

    async fn decode(identifier: String) -> Result<Self, DiskDataError> {
        let mut reader = File::open(Self::path(identifier))?;
        let mut pem_bytes = Vec::new();
        reader.read_to_end(&mut pem_bytes)?;
        let pem = String::from_utf8(pem_bytes).unwrap();
        let key = SigningKey::from_pkcs8_pem(&pem).unwrap();
        Ok(key)
    }
}
