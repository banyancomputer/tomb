mod io;

//pub(crate) mod testing;
pub use io::{get_read, get_read_write, get_write};

#[cfg(test)]
//pub use io::compute_directory_size;
mod cast;
pub mod varint;

mod error;
pub(crate) use error::UtilityError;
use indicatif::{ProgressBar, ProgressStyle};

/// Create a progress bar for displaying progress through a task with a predetermined style
pub fn get_progress_bar(count: u64) -> ProgressBar {
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = ProgressBar::new(count);
    // Stylize that progress bar!
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
            )
            .unwrap(),
    );
    progress_bar
}
