use std::path::PathBuf;

use crate::{
    utils::{all_bfs_paths, is_visible},
    NativeError,
};
use banyanfs::{
    filesystem::Drive,
    stores::DataStore,
    utils::{calculate_cid, crypto_rng},
};
use tokio::io::AsyncReadExt;
use tracing::{info, warn};
use walkdir::WalkDir;

pub async fn prepare(
    drive: &mut Drive,
    store: &mut impl DataStore,
    path: &PathBuf,
) -> Result<(), NativeError> {
    let mut root = drive.root().await?;
    let mut rng = crypto_rng();

    // Iterate over every entry in the FileSystem
    for bfs_path in all_bfs_paths(drive).await? {
        // Deterministically canonicalize the path
        if !path.join(&bfs_path).exists() {
            warn!(
                "{} was present in the FS but not on disk. Deleting.",
                bfs_path.display()
            );
            let bfs_path: Vec<&str> = bfs_path
                .components()
                .filter_map(|v| v.as_os_str().to_str())
                .collect();
            root.rm(store, &bfs_path).await.ok();
        }
    }

    // Iterate over every entry on disk
    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_entry(is_visible)
    {
        match entry {
            Ok(entry) => {
                // Path on OS
                let canonical_path = entry.path();
                // Banyanfs path relative to root
                let bfs_path = canonical_path.strip_prefix(path)?;
                let bfs_path: Vec<&str> = bfs_path
                    .components()
                    .filter_map(|v| v.as_os_str().to_str())
                    .collect();

                if !bfs_path.is_empty() {
                    // If directory
                    if canonical_path.is_dir() {
                        root.mkdir(&mut rng, &bfs_path, true).await?;
                    }
                    // If file
                    else {
                        // Read in the data
                        let mut data = Vec::new();
                        let mut file = tokio::fs::File::open(&canonical_path).await?;
                        file.read_to_end(&mut data).await?;

                        match root.cid(&bfs_path).await {
                            // There is a file and it has a cid
                            Ok(plaintext_cid) => {
                                // Remove and rewrite the file if it has changed
                                if calculate_cid(&data) != plaintext_cid {
                                    info!("File at path {bfs_path:?} has been modified and is being rewritten.");
                                    root.rm(store, &bfs_path).await.ok();
                                    root.write(&mut rng, store, &bfs_path, &data).await?;
                                }
                            }
                            // Assume this failed because the path doesn't exist in the filesystem
                            // TODO improve resilience?
                            Err(_) => {
                                info!("Writing new file at {bfs_path:?}");
                                root.write(&mut rng, store, &bfs_path, &data).await?;
                            }
                        }
                    }
                }
            }
            Err(err) => {
                warn!("Unable to process file or directory, you might not have permission to. {err:?}");
            }
        }
    }

    info!("finished preparing");

    Ok(())
}
