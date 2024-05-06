//! This crate contains all modules in our project.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]

/// CLI Parsing
pub mod cli;

/// Drive structs
pub mod drive;

pub mod file_scanning;
pub mod operations;

/// Local share data
pub mod on_disk;

///
pub mod utils;

/// Error
mod error;
pub use error::*;
