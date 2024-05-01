use super::super::*;
use async_trait::async_trait;
use banyanfs::prelude::*;
use std::{
    fs::File,
    io::{Read, Write},
};

#[async_trait(?Send)]
impl DiskData for SigningKey {
    const TYPE: DataType = DataType::LocalShare;
    const SUFFIX: &'static str = "user_keys";
    const EXTENSION: &'static str = "pem";

    async fn encode(&self, identifier: String) -> Result<(), DiskDataError> {
        let mut writer = File::create(Self::path(identifier))?;
        let pem: String = self.to_pkcs8_pem().unwrap().to_string();
        writer.write_all(pem.as_bytes())?;
        return Ok(());
    }

    async fn decode(identifier: String) -> Result<Self, DiskDataError> {
        let mut reader = File::open(Self::path(identifier))?;
        let mut pem_bytes = Vec::new();
        reader.read_to_end(&mut pem_bytes)?;
        let pem = String::from_utf8(pem_bytes).unwrap();
        let key = SigningKey::from_pkcs8_pem(&pem).unwrap();
        return Ok(key);
    }
    }
