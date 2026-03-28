//! Repo tree - rt: local repository manager.
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod cli;
mod config;
mod error;
mod git;
mod jujutsu;
mod prompt_builder;
mod repo_id;
mod repo_state;
mod repository;
mod utils;
mod version_control_system;

pub use crate::cli::run;
pub use crate::config::Config;
pub use crate::error::NotImplementedError;
pub use crate::prompt_builder::PromptBuilder;
pub use crate::repo_id::Host;
pub use crate::repo_id::RepoId;
pub use crate::repo_state::RepoState;
pub use crate::repository::Repositories;
pub use crate::repository::Repository;
pub use crate::repository::load_empty_dirs;
pub use crate::version_control_system::VersionControlSystem;
