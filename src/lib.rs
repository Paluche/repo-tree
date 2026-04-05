//! Repo tree - rt: local repository manager.
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod cli;
mod config;
mod error;
mod git;
mod host;
mod jujutsu;
mod prompt_builder;
mod repo_id;
mod repo_state;
mod repository;
mod resolve;
mod utils;
mod version_control_system;

pub use cli::run;
