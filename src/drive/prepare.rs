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
};

impl DiskDriveAndStore {
    pub async fn prepare(&mut self, origin: &PathBuf) -> Result<(), OnDiskError> {
        let mut root = self.drive.root().await.unwrap();
        let mut rng = crypto_rng();

        // Iterate over every entry in the FileSystem
        //let root_entry = self.drive.root_entry().await.unwrap();
        for path in self.enumerate_paths(Path::new(""), &root).await? {
            // Deterministically canonicalize the path
            let canon = origin.join(&path);
            //
            if !canon.exists() {
                warn!("{} was present in the FS but not on disk.", path.display());
                if prompt_for_bool("Delete?") {
                    let bfs_path: Vec<&str> = path
                        .components()
                        .map(|v| v.as_os_str().to_str().unwrap())
                        .collect();

                    root.rm(&mut self.store, &bfs_path).await.unwrap();
                }
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
                    let bfs_path = canonical_path.strip_prefix(origin).unwrap();
                    info!("canonical: {:?}", canonical_path);
                    let bfs_path: Vec<&str> = bfs_path
                        .components()
                        .map(|v| v.as_os_str().to_str().unwrap())
                        .collect();
                    info!("bfs: {:?}", bfs_path);

                    if !bfs_path.is_empty() {
                        // If directory
                        if canonical_path.is_dir() {
                            root.mkdir(&mut rng, &bfs_path, true).await.unwrap();
                        }
                        // If file
                        else {
                            // Read in the data
                            let mut data = Vec::new();
                            let mut file = tokio::fs::File::open(&canonical_path).await.unwrap();
                            file.read_to_end(&mut data).await?;
                            root.write(&mut rng, &mut self.store, &bfs_path, &data)
                                .await
                                .unwrap();
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

    /*
    pub async fn enumerate_paths(
        &self,
        root: &DirectoryHandle,
        prefix: &Path,
        entry: DirectoryEntry,
    ) -> Result<Vec<PathBuf>, OnDiskError> {
        let mut paths = Vec::new();

        match entry.kind() {
            NodeKind::File => paths.push(prefix.join(entry.name().to_string())),
            NodeKind::Directory => {
                let
                let handle = root.
            }
            _ => {}
        }

        Ok(paths)
    }
    */

    #[async_recursion]
    pub async fn enumerate_paths(
        &self,
        prefix: &Path,
        handle: &DirectoryHandle,
    ) -> Result<Vec<PathBuf>, OnDiskError> {
        let mut paths = Vec::new();

        for entry in handle.ls(&[]).await.unwrap() {
            let name = entry.name().to_string();
            let new_prefix = prefix.join(&name);

            match entry.kind() {
                NodeKind::File => {
                    paths.push(new_prefix);
                }
                NodeKind::Directory => {
                    let new_handle = handle.cd(&[&name]).await.unwrap();
                    paths.extend(self.enumerate_paths(&new_prefix, &new_handle).await?);
                }
                _ => {}
            }
        }

        Ok(paths)
    }
}
/*
pub async fn create_plans(origin: &Path, follow_links: bool) -> Result<Vec<PreparationPlan>, ()> {
    // HashSet to track files that have already been seen
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    // Vector holding all the PreparePipelinePlans for bundling
    let mut bundling_plan: Vec<PreparationPlan> = vec![];

    info!("🔍 Deduplicating the filesystem at {}", origin.display());
    // Group the filesystem provided to detect duplicates
    let group_plans = grouper(origin, follow_links, &mut seen_files).unwrap();
    // Extend the bundling plan
    bundling_plan.extend(group_plans);

    // TODO fix setting follow_links / do it right
    info!(
        "📁 Finding directories and symlinks to back up starting at {}",
        origin.display()
    );

    // Spider the filesystem provided to include directories and symlinks
    let spidered_files = spider(origin, follow_links, &mut seen_files).await.unwrap();
    // Extend the bundling plan
    bundling_plan.extend(spidered_files);

    info!(
        "💾 Total number of files to prepare: {}",
        bundling_plan.len()
    );

    Ok(bundling_plan)
}

/// Given a set of PreparePipelinePlans and required structs, process each
pub async fn process_plans(
    ddas: &mut DiskDriveAndStore,
    preparation_plan: Vec<PreparationPlan>,
) -> Result<(), DiskDataError> {
    let mut root = ddas.drive.root().await.unwrap();
    let mut rng = crypto_rng();

    // First, write data which corresponds to real data
    for plan in preparation_plan {
        match plan {
            PreparationPlan::FileGroup(metadatas) => {
                // Load the file from disk
                let mut file = File::open(&metadatas[0].canonicalized_path).await?;
                let mut content = <Vec<u8>>::new();
                file.read_to_end(&mut content).await?;

                root.write(
                    &mut rng,
                    &mut ddas.store,
                    &metadatas[0]
                        .bfs_path
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>(),
                    &content,
                );

                // Duplicates need to be linked no matter what
                for meta in &metadatas[1..] {
                    // TODO
                }
            }
            // If this is a directory or symlink
            PreparationPlan::Directory(meta) => {
                // If the directory does not exist
                root.mkdir(
                    &mut rng,
                    &meta
                        .bfs_path
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<&str>>(),
                    true,
                )
                .await
                .unwrap();
            }
            PreparationPlan::Symlink(_, _) => todo!("not sure on banyanfs"),
        }
    }

    // Return Ok
    Ok(())
}
*/
