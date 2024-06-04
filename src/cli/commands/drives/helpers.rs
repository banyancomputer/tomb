use std::path::PathBuf;

use banyanfs::{
    api::{
        platform::{self, ApiDrive},
        VecStream,
    },
    codec::{
        crypto::{SigningKey, VerifyingKey},
        header::{AccessMaskBuilder, ContentOptions},
    },
    filesystem::DriveLoader,
    stores::{SyncTracker, SyncableDataStore},
    utils::crypto_rng,
};
use futures::{io::Cursor, StreamExt};
use tokio::fs::create_dir_all;
use tracing::{error, info, warn};

use crate::{
    cli::commands::drives::CborSyncTracker,
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        local_share::DriveAndKeyId,
        OnDisk,
    },
    utils::prompt_for_bool,
    NativeError,
};

use super::LocalBanyanFS;

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
                error!("Logged in, but failed to fetch platform drives. {err}");
                vec![]
            }
        },
        Err(_) => {
            warn!("You aren't logged in. Login to see platform drives.");
            vec![]
        }
    }
}
