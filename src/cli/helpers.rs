use banyanfs::{
    api::{
        api_fingerprint_key,
        platform::{self, ApiDrive, ApiUserKey},
    },
    codec::crypto::{SigningKey, VerifyingKey},
};
use tracing::{error, warn};

use crate::{
    on_disk::{config::GlobalConfig, OnDisk},
    ConfigStateError, NativeError,
};

pub async fn platform_drive_with_name(global: &GlobalConfig, name: &str) -> Option<ApiDrive> {
    platform_drives(global)
        .await
        .into_iter()
        .find(|platform_drive| platform_drive.name == name)
}

pub async fn platform_drives(global: &GlobalConfig) -> Vec<ApiDrive> {
    match global.get_client().await {
        Ok(client) => match platform::drives::get_all(&client).await {
            Ok(d) => d,
            Err(_) => {
                warn!("Logged in, but failed to fetch platform drives.");
                vec![]
            }
        },
        Err(_) => {
            warn!("You aren't logged in. Login to see platform drives.");
            vec![]
        }
    }
}

pub async fn platform_user_keys(global: &GlobalConfig) -> Vec<ApiUserKey> {
    match global.get_client().await {
        Ok(client) => match platform::account::user_key_access(&client).await {
            Ok(d) => d.into_iter().map(|uka| uka.key).collect(),
            Err(_) => {
                warn!("Logged in, but failed to fetch platform drives.");
                vec![]
            }
        },
        Err(_) => {
            warn!("You aren't logged in. Login to see platform user keys.");
            vec![]
        }
    }
}

pub async fn public_key_and_fingerprint(
    global: &GlobalConfig,
    name: &String,
) -> Result<(VerifyingKey, String), NativeError> {
    if let Ok(user_key) = SigningKey::decode(name).await {
        let public_key = user_key.verifying_key();
        let fingerprint = api_fingerprint_key(&public_key);
        Ok((public_key, fingerprint))
    } else {
        match self::platform_user_keys(&global)
            .await
            .into_iter()
            .find(|key| key.name() == name)
        {
            Some(api_key) => {
                let fingerprint = api_key.fingerprint().to_string();
                let public_key_pem = api_key.public_key();
                let public_key = VerifyingKey::from_spki(&public_key_pem)
                    .map_err(|_| NativeError::Custom("Decode SPKI".into()))?;
                Ok((public_key, fingerprint))
            }
            None => {
                error!("No known user key with that name locally or remotely.");
                Err(NativeError::Custom("missing usrkey".into()).into())
            }
        }
    }
}
