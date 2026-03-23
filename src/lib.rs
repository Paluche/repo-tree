//! Libraries for the repo-tools utils
mod cli;
mod config;
mod error;
mod git;
mod jujutsu;
mod repo_state;
mod repository;
mod url_parser;
mod version_control_system;

use std::path::PathBuf;

pub use crate::{
    cli::run,
    config::{Config, Host},
    error::NotImplementedError,
    repo_state::RepoState,
    repository::Repository,
    url_parser::{parse_repo_url, parse_url},
    version_control_system::VersionControlSystem,
};

/// Load all the repositories present in the repo tree.
/// Print a warning message if empty directories outside any repository are
/// found in the repo tree.
pub fn load_repositories(config: &Config) -> Vec<Repository> {
    let (repositories, empty_dirs) = repository::search(config);

    for empty_dir in empty_dirs {
        eprintln!("Empty directory in REPO_TREE_DIR: {}", empty_dir.display());
    }

    repositories
}

/// Load all the repositories present in the repo tree.
pub fn load_repositories_silent(config: &Config) -> Vec<Repository> {
    repository::search(config).0
}

/// Load some of the repositories based on the provided filters.
pub fn load_filtered_repositories(
    config: &Config,
    filter_hosts: Vec<String>,
    filter_names: Vec<String>,
) -> Vec<Repository> {
    let repositories = load_repositories(config);

    repositories
        .into_iter()
        .filter(|r| {
            (filter_hosts.is_empty()
                || filter_hosts.iter().any(|host| {
                    r.id.host
                        .clone()
                        .is_some_and(|repo_host| &repo_host.name == host)
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
