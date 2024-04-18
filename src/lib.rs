//! This crate contains all modules in our project. TODO(organizedgrime) write something useful here.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms)]

// we dont need car filing in the client anymore i dont think
//pub(crate) mod car;

/// CLI Parsing
pub mod cli;

pub mod drive;
/// Ways of referring to and accessing drives
//pub(crate) mod drive;

/// Native operations
pub mod native;

/// Internal utils
pub(crate) mod utils;

// pub mod prelude { }
