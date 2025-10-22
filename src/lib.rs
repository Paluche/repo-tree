//! Libraries for the repo-tools utils
//!
mod clean_action;
mod git;
mod jujutsu;
mod prompt_action;
mod repository;
mod resolve_action;
mod status_action;
mod url_parser;
mod version_control_system;

pub use crate::{
    clean_action::clean,
    prompt_action::prompt,
    resolve_action::{resolve, resolve_completer},
    status_action::status,
    url_parser::UrlParser,
};

use crate::repository::Repository;

use std::{
    env,
    path::{Path, PathBuf},
};

pub fn get_work_dir() -> PathBuf {
    let ret = PathBuf::from(
        &env::var("WORK_DIR").expect("Missing WORK_DIR environment variable"),
    );

    assert!(ret.is_absolute(), "WORK_DIR value must be an absolute path");

    ret
}

pub fn load_workspace() -> (Vec<Repository>, Vec<PathBuf>) {
    repository::search(Path::new(&get_work_dir()), &UrlParser::default())
}
