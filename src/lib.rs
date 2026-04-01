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
mod version_control_system;

use std::path::PathBuf;

pub use crate::cli::run;
pub use crate::config::Config;
pub use crate::error::NotImplementedError;
pub use crate::prompt_builder::PromptBuilder;
pub use crate::repo_id::Host;
pub use crate::repo_id::RepoId;
pub use crate::repo_state::RepoState;
pub use crate::repository::Repository;
pub use crate::version_control_system::VersionControlSystem;

/// Load all the repositories present in the repo tree.
/// Print a warning message if empty directories outside any repository are
/// found in the repo tree.
pub fn load_repositories(config: &Config) -> Vec<Repository<'_>> {
    let (repositories, empty_dirs) = repository::search(config);

    for empty_dir in empty_dirs {
        eprintln!("Empty directory in REPO_TREE_DIR: {}", empty_dir.display());
    }

    repositories
}

/// Load all the repositories present in the repo tree.
pub fn load_repositories_silent(config: &Config) -> Vec<Repository<'_>> {
    repository::search(config).0
}

/// Load some of the repositories based on the provided filters.
pub fn load_filtered_repositories(
    config: &Config,
    filter_hosts: Vec<String>,
    filter_names: Vec<String>,
) -> Vec<Repository<'_>> {
    let repositories = load_repositories(config);

    repositories
        .into_iter()
        .filter(|r| {
            (filter_hosts.is_empty()
                || filter_hosts.iter().any(|host| match r.id.host.name() {
                    Ok(name) => name == host,
                    Err(err) => {
                        eprintln!("{err}");
                        false
                    }
                }))
                && (filter_names.is_empty()
                    || filter_names
                        .iter()
                        .any(|name| r.id.name.starts_with(name)))
        })
        .collect()
}

/// Search for empty directories outside any repository are found in the repo
/// tree.
pub fn load_empty_dirs(config: &Config) -> Vec<PathBuf> {
    repository::search(config).1
}
