use std::path::PathBuf;

use crate::{
    utils::{all_bfs_paths, is_visible},
    NativeError,
};
use banyanfs::{filesystem::Drive, stores::DataStore, utils::crypto_rng};
use tokio::io::AsyncReadExt;
use tracing::{info, warn};
use walkdir::WalkDir;

pub async fn prepare(
    drive: &mut Drive,
    store: &mut impl DataStore,
    origin: &PathBuf,
) -> Result<(), NativeError> {
    let mut root = drive.root().await?;
    let mut rng = crypto_rng();

    // Iterate over every entry in the FileSystem
    for path in all_bfs_paths(drive).await? {
        //info!("enumerated: {:?}", path);
        // Deterministically canonicalize the path
        let canon = origin.join(&path);
        //
        if !canon.exists() {
            warn!(
                "{} was present in the FS but not on disk. Deleting.",
                path.display()
            );
            //if prompt_for_bool("Delete?") {
            let bfs_path: Vec<&str> = path
                .components()
                .filter_map(|v| v.as_os_str().to_str())
                .collect();

            root.rm(store, &bfs_path).await.ok();
            //}
        }
    }

    // Iterate over every entry on disk
    for entry in WalkDir::new(origin)
        .follow_links(true)
        .into_iter()
        .filter_entry(is_visible)
    {
        match entry {
            Ok(entry) => {
                // Path on OS
                let canonical_path = entry.path();
                // Banyanfs path relative to root
                let bfs_path = canonical_path.strip_prefix(origin)?;
                let bfs_path: Vec<&str> = bfs_path
                    .components()
                    .filter_map(|v| v.as_os_str().to_str())
                    .collect();

                if !bfs_path.is_empty() {
                    info!("canonical: {:?}", canonical_path);
                    info!("bfs: {:?}", bfs_path);

                    // If directory
                    if canonical_path.is_dir() {
                        info!("making dir");
                        root.mkdir(&mut rng, &bfs_path, true).await?;
                    }
                    // If file
                    else {
                        info!("making file");

                        // Read in the data
                        let mut data = Vec::new();
                        let mut file = tokio::fs::File::open(&canonical_path).await?;
                        file.read_to_end(&mut data).await?;
                        // TODO this kinda sucks.
                        root.rm(store, &bfs_path).await.ok();
                        // Write doesn't work unless the thing isn't already there
                        root.write(&mut rng, store, &bfs_path, &data).await?;
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
