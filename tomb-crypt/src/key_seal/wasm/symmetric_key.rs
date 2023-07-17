use async_trait::async_trait;

use crate::key_seal::KeySealError;
use crate::key_seal::common::*;
use crate::key_seal::wasm::*;
use crate::key_seal::EncryptedSymmetricKey;
use crate::key_seal::EcPublicEncryptionKey;

pub struct SymmetricKey(pub(crate) [u8; AES_KEY_SIZE]);


#[async_trait(?Send)]
impl PlainKey for SymmetricKey {
    type Error = KeySealError;
    type ProtectedKey = EncryptedSymmetricKey;
    type WrappingPublicKey = EcPublicEncryptionKey;

    async fn encrypt_for(
        &self,
        recipient_key: &Self::WrappingPublicKey,
    ) -> Result<Self::ProtectedKey, KeySealError> {
        todo!();
        // let ephemeral_key = EcEncryptionKey::generate()?;

        // let ecdh_shared_secret = internal::ecdh_exchange(&ephemeral_key.0, &recipient_key.0);

        // let info = internal::generate_info(
        //     ephemeral_key.fingerprint()?.as_ref(),
        //     recipient_key.fingerprint()?.as_ref(),
        // );
        // let (salt, hkdf_shared_secret) = internal::hkdf(&ecdh_shared_secret, &info);

        // let encrypted_key = internal::wrap_key(&hkdf_shared_secret, &self.0);
        // let exported_ephemeral_key = ephemeral_key.public_key()?.export_bytes()?;

        // Ok(EncryptedSymmetricKey {
        //     data: encrypted_key,
        //     salt,
        //     public_key: exported_ephemeral_key,
        // })
    }
}

//impl SymmetricKey {
//    #[cfg(test)]
//    fn generate() -> Self {
//        let mut key_data = [0u8; AES_KEY_SIZE];
//        openssl::rand::rand_bytes(&mut key_data).expect("unable to generate key data");
//        Self(key_data)
//    }
//}

impl AsRef<[u8]> for SymmetricKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; AES_KEY_SIZE]> for SymmetricKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}
