use banyanfs::{
    api::{
        api_fingerprint_key,
        platform::{self, ApiDrive, ApiUserKey},
    },
    codec::crypto::{SigningKey, VerifyingKey},
};
use tracing::{error, warn};

use crate::{
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk,
    },
    ConfigStateError, NativeError,
};

impl GlobalConfig {
    pub async fn drive_platform_id(&mut self, drive_id: &str) -> Result<String, NativeError> {
        if let Some(platform_id) = self.drive_platform_ids.get(drive_id) {
            return Ok(platform_id.to_string());
        }
        let client = self.get_client().await?;
        let drive_platform_id = platform::drives::get_all(&client)
            .await?
            .into_iter()
            .find(|drive| drive.name == drive_id)
            .ok_or(ConfigStateError::MissingDrive(drive_id.into()))?
            .id;
        self.drive_platform_ids
            .insert(drive_id.to_string(), drive_platform_id.clone());
        self.encode(&GlobalConfigId).await?;
        Ok(drive_platform_id)
    }

    pub async fn platform_drive_with_name(&self, name: &str) -> Option<ApiDrive> {
        self.platform_drives()
            .await
            .into_iter()
            .find(|platform_drive| platform_drive.name == name)
    }

    pub async fn platform_drives(&self) -> Vec<ApiDrive> {
        match self.get_client().await {
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

    pub async fn platform_user_keys(&self) -> Vec<ApiUserKey> {
        match self.get_client().await {
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
        &self,
        name: &String,
    ) -> Result<(VerifyingKey, String), NativeError> {
        if let Ok(user_key) = SigningKey::decode(name).await {
            let public_key = user_key.verifying_key();
            let fingerprint = api_fingerprint_key(&public_key);
            Ok((public_key, fingerprint))
        } else {
            match self
                .platform_user_keys()
                .await
                .into_iter()
                .find(|key| key.name() == name)
            {
                Some(api_key) => {
                    let fingerprint = api_key.fingerprint().to_string();
                    let public_key_pem = api_key.public_key();
                    let public_key = VerifyingKey::from_spki(public_key_pem)
                        .map_err(|_| NativeError::Custom("Decode SPKI".into()))?;
                    Ok((public_key, fingerprint))
                }
                None => {
                    error!("No known user key with that name locally or remotely.");
                    Err(NativeError::Custom("missing usrkey".into()))
                }
            }
        }
    }
}
