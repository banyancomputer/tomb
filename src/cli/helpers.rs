use banyanfs::api::platform::{self, ApiDrive, ApiUserKey};
use tracing::warn;

use crate::on_disk::config::GlobalConfig;

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
