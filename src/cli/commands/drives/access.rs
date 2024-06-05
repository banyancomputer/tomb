use std::collections::HashMap;

use async_trait::async_trait;
use banyanfs::prelude::*;
use banyanfs::utils::crypto_rng;
use banyanfs::{api::api_fingerprint_key, codec::crypto::SigningKey};
use clap::Subcommand;
use cli_table::{print_stdout, Cell, Table};
use tracing::{error, info};

use crate::cli::commands::drives::LocalBanyanFS;
use crate::{
    cli::RunnableCommand,
    on_disk::{config::GlobalConfig, local_share::DriveAndKeyId, OnDisk},
    NativeError,
};

use super::DriveOperationPayload;

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DriveAccessCommand {
    /// List drive actors
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
impl RunnableCommand<NativeError> for DriveAccessCommand {
    type Payload = DriveOperationPayload;

    async fn run(self, mut payload: DriveOperationPayload) -> Result<(), NativeError> {
        use DriveAccessCommand::*;
        match self {
            Ls => {
                let mut key_names = HashMap::new();
                for name in SigningKey::entries() {
                    key_names.insert(
                        api_fingerprint_key(&SigningKey::decode(&name).await?.verifying_key()),
                        name,
                    );
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
            DriveAccessCommand::Grant { name } => {
                let user_key = SigningKey::decode(&name).await?;
                let public_user_key = user_key.verifying_key();
                let fingerprint = api_fingerprint_key(&public_user_key);
                let bfs = LocalBanyanFS::decode(&payload.id).await?;
                let access_mask = AccessMaskBuilder::full_access().build()?;
                if let Some((_, _mask)) =
                    bfs.drive.verifying_keys().await.iter().find(|(key, mask)| {
                        api_fingerprint_key(key) == fingerprint && *mask == access_mask
                    })
                {
                    error!("That key has already been granted access to this Drive!");
                } else {
                    bfs.drive
                        .authorize_key(&mut crypto_rng(), public_user_key, access_mask)
                        .await?;
                    bfs.encode(&payload.id).await?;
                    info!("<< GRANTED LOCAL ACCESS FOR USER KEY >>");
                    payload.sync().await?;
                    info!("<< GRANTED PLATFORM ACCESS FOR USER KEY >>");
                }

                Ok(())
            }
            DriveAccessCommand::Revoke { name } => {
                let user_key = SigningKey::decode(&name).await?;
                let public_user_key = user_key.verifying_key();
                let fingerprint = api_fingerprint_key(&public_user_key);
                let bfs = LocalBanyanFS::decode(&payload.id).await?;
                let access_mask = AccessMaskBuilder::full_access().historical().build()?;

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
                        bfs.drive
                            .authorize_key(&mut crypto_rng(), public_user_key, access_mask)
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
