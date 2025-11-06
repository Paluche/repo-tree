//! Libraries for the repo-tools utils
pub mod cli;
mod config;
mod git;
mod jujutsu;
mod repository;
mod url_parser;
mod version_control_system;

pub use crate::{
    config::{Config, Host},
    repository::Repository,
    url_parser::UrlParser,
    version_control_system::VersionControlSystem,
};
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn get_workspace_dir() -> PathBuf {
    let ret = PathBuf::from(
        &env::var("WORKSPACE_DIR")
            .expect("Missing WORKSPACE_DIR environment variable"),
    );

    assert!(
        ret.is_absolute(),
        "WORKSPACE_DIR value must be an absolute path"
    );

    ret
}

pub fn load_workspace(
    workspace_dir: &Path,
    url_parser: &UrlParser,
) -> (Vec<Repository>, Vec<PathBuf>) {
    repository::search(workspace_dir, url_parser)
}
