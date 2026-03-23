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

use std::{
    env,
    path::{Path, PathBuf},
};

pub use crate::{
    cli::run,
    config::{Config, Host},
    error::NotImplementedError,
    repo_state::RepoState,
    repository::Repository,
    url_parser::UrlParser,
    version_control_system::VersionControlSystem,
};

pub fn iter_repos_from(
    repositories: Vec<Repository>,
    start: Option<Repository>,
) -> Box<dyn DoubleEndedIterator<Item = Repository>> {
    if let Some(start) = start {
        // Use partition_in_place when stable.
        let mut start_found = false;
        let (start, end): (Vec<Repository>, Vec<Repository>) =
            repositories.into_iter().partition(move |r| {
                if r == &start {
                    start_found = true;
                }
                start_found
            });

        Box::new(start.into_iter().skip(1).chain(end))
    } else {
        Box::new(repositories.into_iter())
    }
}

pub fn get_repo_tree_dir() -> PathBuf {
    let ret = PathBuf::from(
        &env::var("REPO_TREE_DIR")
            .expect("Missing REPO_TREE_DIR environment variable"),
    );

    assert!(
        ret.is_absolute(),
        "REPO_TREE_DIR value must be an absolute path"
    );

    ret
}

pub fn load_repositories(
    repo_tree_dir: &Path,
    url_parser: &UrlParser,
) -> Vec<Repository> {
    let (repositories, empty_dirs) =
        repository::search(repo_tree_dir, url_parser);

    for empty_dir in empty_dirs {
        eprintln!("Empty directory in REPO_TREE_DIR: {}", empty_dir.display());
    }

    repositories
}

pub fn load_repositories_silent(
    repo_tree_dir: &Path,
    url_parser: &UrlParser,
) -> Vec<Repository> {
    repository::search(repo_tree_dir, url_parser).0
}

pub fn load_filtered_repositories(
    repo_tree_dir: &Path,
    url_parser: &UrlParser,
    filter_hosts: Vec<String>,
    filter_names: Vec<String>,
) -> Vec<Repository> {
    let repositories = load_repositories(repo_tree_dir, url_parser);

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

pub fn load_empty_dirs(
    repo_tree_dir: &Path,
    url_parser: &UrlParser,
) -> Vec<PathBuf> {
    repository::search(repo_tree_dir, url_parser).1
}
