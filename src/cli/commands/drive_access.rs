use std::collections::HashMap;

use async_trait::async_trait;
use banyanfs::{
    api::{api_fingerprint_key, platform},
    codec::crypto::SigningKey,
};
use clap::Subcommand;
use cli_table::{print_stdout, Cell, Table};
use tracing::warn;

use crate::{
    cli::{
        specifiers::{DriveId, DriveSpecifier},
        RunnableCommand,
    },
    drive::local::LocalLoadedDrive,
    on_disk::{
        config::{GlobalConfig, GlobalConfigId},
        OnDisk, OnDiskExt,
    },
    NativeError,
};

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DriveAccessCommand {
    /// List drive actors
    Ls(DriveSpecifier),
    /// Grant access to a known key
    Grant,
    /// Revoke access from a known key
    Revoke,
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DriveAccessCommand {
    async fn run_internal(self) -> Result<(), NativeError> {
        let global = GlobalConfig::decode(&GlobalConfigId).await?;
        match self {
            DriveAccessCommand::Ls(ds) => {
                let mut key_names = HashMap::new();
                for name in SigningKey::entries() {
                    key_names.insert(
                        api_fingerprint_key(&SigningKey::decode(&name).await?.verifying_key()),
                        name,
                    );
                }

                let loaded = LocalLoadedDrive::load(&ds.into(), &global).await?;
                let keys = loaded.bfs.drive.verifying_keys().await;
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
            DriveAccessCommand::Grant => Ok(()),
            DriveAccessCommand::Revoke => Ok(()),
        }
    }
}
