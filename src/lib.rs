//! Libraries for the repo-tools utils
pub mod cli;
mod config;
mod git;
mod jujutsu;
mod repository;
mod url_parser;
mod version_control_system;

use std::{
    env,
    path::{Path, PathBuf},
};

pub use crate::{
    config::{Config, Host},
    repository::Repository,
    url_parser::UrlParser,
    version_control_system::VersionControlSystem,
};

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
    load_repositories(repo_tree_dir, url_parser)
        .into_iter()
        .filter(|r| {
            filter_hosts.iter().any(|host| {
                if let Some(repo_host) = &r.id.host {
                    &repo_host.name == host
                } else {
                    host == "local"
                }
            }) && filter_names.iter().any(|name| r.id.name.starts_with(name))
        })
        .collect()
}

pub fn load_empty_dirs(
    repo_tree_dir: &Path,
    url_parser: &UrlParser,
) -> Vec<PathBuf> {
    repository::search(repo_tree_dir, url_parser).1
}
