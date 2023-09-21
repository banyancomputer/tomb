use crate::types::spider::{BundlePipelinePlan, SpiderMetadata};
use anyhow::Result;
use jwalk::WalkDir;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Walks the input directory and returns a list of all the files and directories in it.
/// # Arguments
/// * `origin` - The path to the directory to be walked.
/// * `follow_links` - Whether or not to follow symlinks. (currently not supported)
/// # Returns
/// A `Result`, which can either succeed or fail. If it succeeds, it returns a vector of SpiderMetadata. If it fails, it returns an error.
// TODO (organizedgrime): add support for following symlinks
pub async fn spider(
    origin: &Path,
    _follow_links: bool,
    seen_files: &mut HashSet<PathBuf>,
) -> Result<Vec<BundlePipelinePlan>> {
    // Canonicalize the path
    let path_root = origin.canonicalize()?;

    // Walk the contents of the input directory and get a list of them
    let walk_dir = WalkDir::new(&path_root)
        // Only follow symlinks if the user specified it
        // TODO support symlinks- right now we are NOT doing this. document this decision!
        .follow_links(false)
        // Process the contents of the directory in parallel
        .process_read_dir(|_depth, _path, _read_dir_state, _children| ());

    let mut bundling_plan = vec![];

    // TODO (laudiacay): make sure handoff from jwalk to tokio is efficient
    // Hand off the iterator generated by WalkDirGeneric to tokio. This turns the iterator into a stream
    let spidered: Vec<SpiderMetadata> = walk_dir
        .into_iter()
        .map(move |item| {
            item.map(|entry| SpiderMetadata::new(&path_root, entry))
                .map_err(|e| e.into())
        })
        .collect::<Result<Vec<SpiderMetadata>>>()?;

    for spidered in spidered.into_iter() {
        // If this is a duplicate
        if seen_files.contains(&spidered.canonicalized_path.to_path_buf()) {
            // Just skip it
            continue;
        }
        // Now that we've checked for duplicates, add this to the seen files
        seen_files.insert(spidered.canonicalized_path.clone());

        // Construct Automatic Reference Counting pointer to the spidered metadata
        let origin_data = Arc::new(spidered.clone());
        // If this is a directory
        if spidered.original_metadata.is_dir() {
            // Push a BundlePipelinePlan with this origin data
            bundling_plan.push(BundlePipelinePlan::Directory(origin_data));
        }
        // If this is a symlink
        else if spidered.original_metadata.is_symlink() {
            // The canon path, as a String
            let canon_path = origin_data
                .canonicalized_path
                .to_str()
                .expect("failed to represent path as string");
            // The suffix of the canon path we'd like to drop
            let canon_ignored_suffix = origin_data
                .original_location
                .to_str()
                .expect("failed to represent path as string");
            // The new canon path has the suffix removed
            let canon_path = canon_path
                .strip_suffix(canon_ignored_suffix)
                .expect("failed to strip suffix");

            // A portion of this canon path will be prefixes of the symlink target that need to be removed
            // Transform the canon path into a set of prefixes
            let prefixes: Vec<String> = canon_path.split('/').map(|x| format!("{}/", x)).collect();

            // Determine where this symlink points to, an operation that should never fail
            let mut symlink_target =
                fs::read_link(&spidered.canonicalized_path).expect("failed to read symlink");

            // For each real prefix (first and last are empty)
            for prefix in &prefixes[1..prefixes.len() - 1] {
                // If we can actually strip that prefix from the symlink target
                if let Ok(new_path) = symlink_target.strip_prefix(prefix) {
                    // Do so
                    symlink_target = new_path.to_path_buf();
                }
                // Otherwise this isn't a prefix anyway, nothing needs to happen
            }

            // Push a BundlePipelinePlan with this origin data and symlink
            bundling_plan.push(BundlePipelinePlan::Symlink(origin_data, symlink_target));
        }
        // If this is a file that was not in a group
        else {
            // Push a BundlePipelinePlan using fake file group of singular spidered metadata
            bundling_plan.push(BundlePipelinePlan::FileGroup(vec![origin_data]));
        }
    }
    Ok(bundling_plan)
}

