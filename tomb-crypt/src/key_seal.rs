mod common;
mod error;

pub use error::KeySealError;

#[cfg(not(target_arch = "wasm"))]
mod standard;

#[cfg(not(target_arch = "wasm"))]
pub use standard::{EcEncryptionKey, EcPublicEncryptionKey, SymmetricKey, EncryptedSymmetricKey};

//#[cfg(target_arch = "wasm")]
//mod wasm;

//#[cfg(target_arch = "wasm")]
//{
//}

pub fn pretty_fingerprint(fingerprint_bytes: &[u8]) -> String {
    fingerprint_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<String>>()
        .join(":")
}
