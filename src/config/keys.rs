use crate::NativeError;
use banyanfs::prelude::*;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

/// Generate a new Ecdsa key to use for authentication
/// Writes the key to the config path
pub async fn new_user_key(path: &PathBuf) -> Result<SigningKey, NativeError> {
    if path.exists() {
        load_user_key(path).await?;
    }
    let mut rng = banyanfs::utils::crypto_rng();
    let key = SigningKey::generate(&mut rng);
    let pem: String = key.to_pkcs8_pem().unwrap().to_string();
    let mut f = File::create(path)?;
    f.write_all(pem.as_bytes())?;
    Ok(key)
}

/// Read the API key from disk
pub async fn load_user_key(path: &PathBuf) -> Result<SigningKey, NativeError> {
    let mut reader = File::open(path)?;
    let mut pem_bytes = Vec::new();
    reader.read_to_end(&mut pem_bytes)?;
    let pem = String::from_utf8(pem_bytes).unwrap();
    let key = SigningKey::from_pkcs8_pem(&pem).unwrap();
    Ok(key)
}

/// Save the API key to disk
pub async fn save_user_key(path: &PathBuf, key: SigningKey) -> Result<(), NativeError> {
    let mut writer = File::create(path)?;
    let pem: String = key.to_pkcs8_pem().unwrap().to_string();
    writer.write_all(pem.as_bytes())?;
    Ok(())
}
