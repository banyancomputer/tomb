//! This crate contains all modules in our project.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]

/// CLI Parsing
pub mod cli;

/// Drive structs
pub mod drive;

/// Configuration
pub mod config;

pub mod utils;

/// Error
mod error;
pub use error::*;
