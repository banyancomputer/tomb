use crate::types::spider::{make_spider_metadata, SpiderMetadata};
use anyhow::Result;
use jwalk::WalkDir;
use std::path::PathBuf;

pub async fn spider(input_dir: PathBuf, _follow_links: bool) -> Result<Vec<SpiderMetadata>> {
    // Canonicalize the path
    let path_root = input_dir.canonicalize()?;

    // Walk the contents of the input directory and get a list of them
    let walk_dir = WalkDir::new(path_root.clone())
        // Only follow symlinks if the user specified it
        // TODO support symlinks- right now we are NOT doing this. document this decision!
        .follow_links(false)
        // Process the contents of the directory in parallel
        .process_read_dir(|_depth, _path, _read_dir_state, _children| ());
    // TODO (laudiacay): make sure handoff from jwalk to tokio is efficient
    // Hand off the iterator generated by WalkDirGeneric to tokio. This turns the iterator into a stream
    walk_dir
        .into_iter()
        .map(move |item| {
            item.map(|d| make_spider_metadata(d, path_root.clone()))
                .map_err(|e| e.into())
        })
        .collect()
}
