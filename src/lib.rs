//! This crate contains all modules in our project. TODO(organizedgrime) write something useful here.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]

// we dont need car filing in the client anymore i dont think
//pub(crate) mod car;

/// CLI Parsing
pub mod cli;

/// Drive structs
pub mod drive;

/// Configuration
pub mod config;

/// Error
mod error;
pub use error::*;
