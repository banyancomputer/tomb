use super::error::TombError;
use crate::{cli::specifiers::BucketSpecifier, types::config::globalconfig::GlobalConfig};
use anyhow::Result;
use std::{fs::File, io::Write, os::unix::fs::symlink, path::Path};
use tomb_common::utils::wnfsio::path_to_segments;
use wnfs::private::PrivateNode;

/// Given the manifest file and a destination for our extracted data, run the extracting pipeline
/// on the data referenced in the manifest.
///
/// # Arguments
///
/// * `output_dir` - &Path representing the relative path of the output directory in which to extract the data
/// * `manifest_file` - &Path representing the relative path of the manifest file
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(
    global: &GlobalConfig,
    bucket_specifier: &BucketSpecifier,
    extracted: &Path,
) -> Result<String, TombError> {
    // Announce that we're starting
    info!("🚀 Starting extracting pipeline...");
    let wrapping_key = global.clone().wrapping_key().await?;
    let config = global.get_bucket_by_specifier(bucket_specifier)?;
    // Load metadata
    let mut fs = config.unlock_fs(&wrapping_key).await?;

    info!(
        "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
        extracted.display()
    );
    // For each node path tuple in the FS Metadata
    for (node, path) in fs.get_all_nodes(&config.metadata).await? {
        match node {
            PrivateNode::Dir(_) => {
                // Create the directory
                std::fs::create_dir_all(extracted.join(path))?;
            }
            PrivateNode::File(file) => {
                let built_path = extracted.join(path.clone());
                // If we can read the content from the file node
                if let Ok(content) = fs
                    .read(&path_to_segments(&path)?, &config.metadata, &config.content)
                    .await
                {
                    // If this file is a symlink
                    if let Some(origin) = file.symlink_origin() {
                        // Write out the symlink
                        symlink(origin, built_path)?;
                    } else {
                        // If the parent does not yet exist
                        if let Some(parent) = built_path.parent() && !parent.exists() {
                            // Create the directories
                            std::fs::create_dir_all(parent)?;
                        }
                        // Create the file at the desired location
                        let mut output_file = File::create(built_path)?;

                        // Write out the content to disk
                        output_file.write_all(&content)?;
                    }
                } else {
                    return Err(TombError::anyhow_error(anyhow::anyhow!(
                        "file missing error"
                    )));
                }
            }
        }
    }

    // Run extraction on the base level with an empty built path
    // process_node(fs, metadata, content, extracted, Path::new("")).await?;

    Ok(format!(
        "successfully extracted data into {}",
        extracted.display()
    ))
}
