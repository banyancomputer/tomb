mod io;
use async_recursion::async_recursion;
use banyanfs::{
    codec::filesystem::NodeKind,
    filesystem::{DirectoryHandle, Drive},
};
use colored::{ColoredString, Colorize};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use tracing::{info, warn};
use uuid::Uuid;
//pub(crate) mod testing;
pub use io::{get_read, get_read_write, get_write};

use crate::NativeError;

#[cfg(test)]
//pub use io::compute_directory_size;
mod cast;

mod error;

pub fn name_of(path: impl AsRef<std::path::Path>) -> Option<String> {
    Some(path.as_ref().file_name()?.to_str()?.to_string())
}

pub fn is_visible(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with("."))
        .unwrap_or(true)
}

#[inline]
fn bool_colorized(value: bool) -> ColoredString {
    if value {
        "Yes".green()
    } else {
        "No".red()
    }
}

/// Prompt the user for a y/n answer
pub fn prompt_for_bool(msg: &str) -> bool {
    info!("{msg} y/n");
    loop {
        let mut input = [0];
        let _ = std::io::stdin().read(&mut input);
        match input[0] as char {
            'y' | 'Y' => return true,
            'n' | 'N' => return false,
            _ => info!("y/n only please."),
        }
    }
}

pub fn prompt_for_uuid(msg: &str) -> String {
    info!("{msg}");
    loop {
        let mut input = String::new();
        while Uuid::parse_str(&input).is_err() {
            if !input.is_empty() {
                warn!("that wasn't a valid UUID.");
            }
            let _ = std::io::stdin().read_line(&mut input);
            input = input.trim().to_string();
        }
        return input;
    }
}

/// Converts a PathBuf into a vector of path segments for use in WNFS.
pub fn path_to_segments(path: impl AsRef<Path>) -> Result<Vec<String>, std::io::Error> {
    let path = path.as_ref().to_path_buf().display().to_string();
    let path_segments: Vec<String> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    Ok(path_segments)
}

/// Enumerates paths in the banyanfs
#[async_recursion]
async fn bfs_paths(prefix: &Path, handle: &DirectoryHandle) -> Result<Vec<PathBuf>, NativeError> {
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
                paths.push(new_prefix.clone());
                paths.extend(bfs_paths(&new_prefix, &new_handle).await?);
            }
            _ => {}
        }
    }

    Ok(paths)
}

pub async fn all_bfs_paths(drive: &Drive) -> Result<Vec<PathBuf>, NativeError> {
    bfs_paths(Path::new(""), &drive.root().await?).await
}
