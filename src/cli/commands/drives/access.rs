use std::collections::HashMap;

use async_trait::async_trait;
use banyanfs::prelude::*;
use banyanfs::utils::crypto_rng;
use banyanfs::{api::api_fingerprint_key, codec::crypto::SigningKey};
use clap::Subcommand;
use cli_table::{print_stdout, Cell, Table};
use tracing::{error, info};

use crate::cli::commands::drives::LocalBanyanFS;
use crate::{cli::RunnableCommand, on_disk::OnDisk, NativeError};

use super::DriveOperationPayload;

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DriveKeyCommand {
    /// List drive keys
    Ls,
    /// Grant access to a known key
    Grant {
        /// Name of the key
        name: String,
    },
    /// Revoke access from a known key
    Revoke {
        /// Name of the key
        name: String,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DriveKeyCommand {
    type Payload = DriveOperationPayload;

    async fn run(self, mut payload: DriveOperationPayload) -> Result<(), NativeError> {
        use DriveKeyCommand::*;
        match self {
            Ls => {
                let mut key_names = HashMap::new();
                for name in SigningKey::entries() {
                    key_names.insert(
                        api_fingerprint_key(&SigningKey::decode(&name).await?.verifying_key()),
                        name,
                    );
                }
                for key in payload.global.platform_user_keys().await {
                    key_names.insert(key.fingerprint().to_string(), key.name().to_string());
                }

                let drive = Drive::decode(&payload.id).await?;
                let keys = drive.verifying_keys().await;
                let mut table_rows = Vec::new();
                for (public_key, mask) in keys {
                    let fingerprint = api_fingerprint_key(&public_key);
                    let name = key_names
                        .get(&fingerprint)
                        .cloned()
                        .unwrap_or("Unknown".to_string());
                    let public_key = public_key.to_spki().unwrap();
                    table_rows.push(vec![
                        name.cell(),
                        fingerprint.cell(),
                        public_key.cell(),
                        mask.is_owner().cell(),
                        mask.has_filesystem_key().cell(),
                        mask.has_maintenance_key().cell(),
                    ])
                }

                let table = table_rows.table().title(vec![
                    "Name".cell(),
                    "Fingerprint".cell(),
                    "Public Key".cell(),
                    "Ownership Access".cell(),
                    "Filesystem Access".cell(),
                    "Maintenance Access".cell(),
                ]);

                print_stdout(table)?;

                Ok(())
            }
            DriveKeyCommand::Grant { name } => {
                // You need to already have access locally to do this
                let bfs = LocalBanyanFS::decode(&payload.id).await?;

                let (public_key, fingerprint) =
                    payload.global.public_key_and_fingerprint(&name).await?;
                // Grab the verifying key and fingerprint, either from disk or Platform
                let access_mask = AccessMaskBuilder::full_access().build()?;
                if let Some((_, _mask)) =
                    bfs.drive.verifying_keys().await.iter().find(|(key, mask)| {
                        api_fingerprint_key(key) == fingerprint && *mask == access_mask
                    })
                {
                    error!("That key has already been granted access to this Drive!");
                } else {
                    bfs.drive
                        .authorize_key(&mut crypto_rng(), public_key, access_mask)
                        .await?;
                    bfs.encode(&payload.id).await?;
                    info!("<< GRANTED LOCAL ACCESS FOR USER KEY >>");
                    payload.sync().await?;
                    info!("<< GRANTED PLATFORM ACCESS FOR USER KEY >>");
                }

                Ok(())
            }
            DriveKeyCommand::Revoke { name } => {
                let bfs = LocalBanyanFS::decode(&payload.id).await?;
                let (public_key, fingerprint) =
                    payload.global.public_key_and_fingerprint(&name).await?;
                if let Some((_, mask)) = bfs
                    .drive
                    .verifying_keys()
                    .await
                    .iter()
                    .find(|(key, _)| api_fingerprint_key(key) == fingerprint)
                {
                    if mask.is_protected() {
                        error!(
                            "This is a protected user key and can not be revoked from the Drive."
                        );
                    } else {
                        let current_key = SigningKey::decode(&payload.id.user_key_id).await?;
                        bfs.drive
                            .remove_key(&current_key, &public_key.actor_id())
                            .await?;
                        bfs.encode(&payload.id).await?;
                        info!("<< REVOKED LOCAL ACCESS FOR USER KEY >>");
                        payload.sync().await?;
                        info!("<< REVOKED PLATFORM ACCESS FOR USER KEY >>");
                    }
                } else {
                    error!("Can't find a user key with that identity in the Drive!");
                }

                Ok(())
            }
        }
    }
}
