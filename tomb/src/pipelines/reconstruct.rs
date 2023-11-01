use super::error::TombError;
use crate::{
    types::config::{bucket::LocalBucket, globalconfig::GlobalConfig},
    utils::wnfsio::get_progress_bar,
};
use anyhow::Result;
use std::{fs::File, io::Write, os::unix::fs::symlink, path::Path};
use tomb_common::utils::wnfsio::path_to_segments;
use wnfs::{common::BlockStore, private::PrivateNode};

/// Given the manifest file and a destination for our restored data, run the restoring pipeline
/// on the data referenced in the manifest.
///
/// # Arguments
///
/// * `output_dir` - &Path representing the relative path of the output directory in which to restore the data
/// * `manifest_file` - &Path representing the relative path of the manifest file
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(
    global: &GlobalConfig,
    local: &LocalBucket,
    content_store: &impl BlockStore,
    restored: &Path,
) -> Result<String, TombError> {
    // Announce that we're starting
    info!("🚀 Starting restoration pipeline...");
    let wrapping_key = global.clone().wrapping_key().await?;
    // Load metadata
    let mut fs = local.unlock_fs(&wrapping_key).await?;
    // Get all the nodes in the FileSystem
    let all_nodes = fs.get_all_nodes(&local.metadata).await?;
    info!(
        "🔐 Restoring all {} files to {}",
        all_nodes.len(),
        restored.display()
    );
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = get_progress_bar(all_nodes.len() as u64)?;
    // For each node path tuple in the FS Metadata
    for (node, path) in all_nodes {
        match node {
            PrivateNode::Dir(_) => {
                // Create the directory
                std::fs::create_dir_all(restored.join(path))?;
                progress_bar.inc(1);
            }
            PrivateNode::File(file) => {
                let built_path = restored.join(path.clone());

                let content = fs
                    .read(&path_to_segments(&path)?, &local.metadata, content_store)
                    .await
                    .map_err(|err| {
                        TombError::custom_error(&format!(
                            "file missing: path: {} & err: {err}",
                            path.display()
                        ))
                    })?;

                // If this file is a symlink
                if let Some(origin) = file.symlink_origin() {
                    // Write out the symlink
                    symlink(origin, built_path)?;
                } else {
                    // If the parent does not yet exist
                    if let Some(parent) = built_path.parent()
                        && !parent.exists()
                    {
                        // Create the directories
                        std::fs::create_dir_all(parent)?;
                    }
                    // Create the file at the desired location
                    let mut output_file = File::create(built_path)?;

                    // Write out the content to disk
                    output_file.write_all(&content)?;
                }

                progress_bar.inc(1);
            }
        }
    }

    Ok(format!(
        "🎉 Data has been successfully reconstructed at this path: {}",
        restored.display()
    ))
}
