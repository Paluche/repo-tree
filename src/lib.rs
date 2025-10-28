//! Libraries for the repo-tools utils
pub mod cli;
mod config;
mod git;
mod jujutsu;
mod repository;
mod url_parser;
mod version_control_system;
mod subversion;

pub use crate::{
    config::Config, repository::Repository, url_parser::UrlParser,
};

use std::{env, path::PathBuf};

pub fn get_work_dir() -> PathBuf {
    let ret = PathBuf::from(
        &env::var("WORK_DIR").expect("Missing WORK_DIR environment variable"),
    );

    assert!(ret.is_absolute(), "WORK_DIR value must be an absolute path");

    ret
}

pub fn load_workspace(
    url_parser: &UrlParser,
) -> (Vec<Repository>, Vec<PathBuf>) {
    repository::search(&get_work_dir(), url_parser)
}
