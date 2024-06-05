use banyanfs::api::platform::{self, ApiDrive};
use tracing::{error, warn};

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
            Err(err) => {
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
