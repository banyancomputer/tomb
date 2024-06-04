use banyanfs::api::platform::{self, ApiDrive};
use tracing::{error, warn};

use crate::on_disk::config::GlobalConfig;

pub async fn api_drive_with_name(global: &GlobalConfig, name: &str) -> Option<ApiDrive> {
    api_drives(global)
        .await
        .into_iter()
        .find(|api| api.name == name)
}

pub async fn api_drives(global: &GlobalConfig) -> Vec<ApiDrive> {
    match global.get_client().await {
        Ok(client) => match platform::drives::get_all(&client).await {
            Ok(d) => d,
            Err(err) => {
                error!("Logged in, but failed to fetch remote drives. {err}");
                vec![]
            }
        },
        Err(_) => {
            warn!("You aren't logged in. Login to see remote drives.");
            vec![]
        }
    }
}
