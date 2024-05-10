mod io;

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
