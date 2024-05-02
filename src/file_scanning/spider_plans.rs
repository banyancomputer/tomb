use jwalk::DirEntry;
use serde::{Deserialize, Serialize};
use std::{
    fs::Metadata,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

#[derive(Debug, Clone)]
/// Metadata associated with a file, directory, or symlink that was processed by the spider
pub struct SpiderMetadata {
    /// Relative path to root for banyanfs
    pub bfs_path: Vec<String>,
    /// canonicalized path
    pub canonicalized_path: PathBuf,
    /// this is the metadata of the original file
    pub metadata: Metadata,
}

impl SpiderMetadata {
    /// Creates a new `SpiderMetadata` struct from a `DirEntry` and a root path.
    /// # Arguments
    /// * `path_root` - The root of the path being spidered
    /// * `entry` - The individual file / directory being processed
    pub fn new(path_root: &Path, entry: DirEntry<((), ())>) -> Self {
        // Determine the location of the entry by stripping the root path from it
        let bfs_path = entry
            .path()
            .strip_prefix(path_root)
            .expect("failed to strip prefix")
            .to_path_buf()
            .display()
            .to_string()
            .split('/')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        // Don't try to canonicalize if this is a symlink
        let mut canonicalized_path: PathBuf = entry.path();
        if !entry.path_is_symlink() {
            canonicalized_path = canonicalized_path
                .canonicalize()
                .expect("failed to canonicalize path")
        };
        // Grab the metadata of the entry
        let metadata = entry.metadata().expect("failed to get entry metadata");
        // Return the SpiderMetadata
        SpiderMetadata {
            bfs_path,
            canonicalized_path,
            metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Enum representing the types of File that the Spider can process.
pub enum FileType {
    /// Directories are files that show us where to find other files.
    Directory,
    /// Symlinks are a special kind of directory.
    Symlink,
    /// Files are just files.
    File,
}

/// This struct is used to describe how a filesystem structure was processed. Either it was a duplicate/symlink/
/// directory and there isn't much to do, or else we need to go through compression, partition, and
/// encryption steps.
/// this takes in pre-grouped files (for processing together) or marked directories/simlinks.
#[derive(Debug, Clone)]
pub enum PreparationPlan {
    /// It was a directory, just create it
    Directory(Arc<SpiderMetadata>),
    /// it was a symlink, just create it (with destination)
    Symlink(Arc<SpiderMetadata>, PathBuf),
    /// it was a group of identical files, here's the metadata for how they were encrypted and compressed
    FileGroup(Vec<Arc<SpiderMetadata>>),
}
