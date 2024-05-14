use std::{
    collections::HashSet,
    ops::Deref,
    path::{Path, PathBuf},
};

use async_recursion::async_recursion;
use banyanfs::{
    codec::filesystem::NodeKind,
    filesystem::{DirectoryEntry, DirectoryHandle},
    utils::crypto_rng,
};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{info, warn};
use walkdir::{DirEntry, WalkDir};

use super::DiskDriveAndStore;
use crate::{
    on_disk::{OnDisk, OnDiskError},
    utils::{is_visible, prompt_for_bool},
    NativeError,
};

impl DiskDriveAndStore {
    pub async fn prepare(&mut self, origin: &PathBuf) -> Result<(), NativeError> {
        let mut root = self.drive.root().await?;
        //self.drive.clean
        let mut rng = crypto_rng();

        // Iterate over every entry in the FileSystem
        for path in Self::enumerate_paths(Path::new(""), &root).await? {
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

                root.rm(&mut self.store, &bfs_path).await.ok();
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
                    info!("canonical: {:?}", canonical_path);
                    let bfs_path: Vec<&str> = bfs_path
                        .components()
                        .filter_map(|v| v.as_os_str().to_str())
                        .collect();
                    info!("bfs: {:?}", bfs_path);

                    if !bfs_path.is_empty() {
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
                            root.write(&mut rng, &mut self.store, &bfs_path, &data)
                                .await?;
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

    #[async_recursion]
    pub async fn enumerate_paths(
        prefix: &Path,
        handle: &DirectoryHandle,
    ) -> Result<Vec<PathBuf>, NativeError> {
        let mut paths = Vec::new();

        for entry in handle.ls(&[]).await? {
            let name = entry.name().to_string();
            let new_prefix = prefix.join(&name);

            match entry.kind() {
                NodeKind::File => {
                    paths.push(new_prefix);
                }
                NodeKind::Directory => {
                    let new_handle = handle.cd(&[&name]).await?;
                    paths.extend(Self::enumerate_paths(&new_prefix, &new_handle).await?);
                }
                _ => {}
            }
        }

        Ok(paths)
    }
}
