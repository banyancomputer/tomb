mod io;
use colored::{ColoredString, Colorize};
use std::io::Read;
use tracing::info;
//pub(crate) mod testing;
pub use io::{get_read, get_read_write, get_write};

#[cfg(test)]
//pub use io::compute_directory_size;
mod cast;

mod error;
pub(crate) use error::UtilityError;

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
