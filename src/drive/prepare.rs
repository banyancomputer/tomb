use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use banyanfs::utils::crypto_rng;
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{info, warn};
use walkdir::{DirEntry, WalkDir};

use super::DiskDriveAndStore;
use crate::{on_disk::OnDiskError, utils::is_visible};

pub async fn prepare(origin: &PathBuf) -> Result<(), OnDiskError> {
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
                let bfs_path = canonical_path.strip_prefix(origin);

                //let file_name = canonical_path.file_name().zip;
                //info!("file_name: {:?}", file_name);
                info!("canonical: {:?}", canonical_path);
                info!("bfs: {:?}", bfs_path);
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
