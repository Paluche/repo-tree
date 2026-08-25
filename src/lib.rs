//! Repo tree - rt: local repository manager.
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod cli;
mod colors;
mod config;
mod error;
mod forge;
mod prompt;
mod repo_id;
mod repo_state;
mod repository;
mod resolve;
mod tree;
mod utils;
mod version_control_system;

pub use cli::run;
