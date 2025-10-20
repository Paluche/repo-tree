//! Libraries for the repo-tools utils
//!
mod git;
mod jujutsu;
mod prompt_action;
mod repository;
mod resolve_action;
mod status_action;
mod url_parser;
mod version_control_system;

pub use crate::{
    prompt_action::prompt,
    resolve_action::{resolve, resolve_completer},
    status_action::status,
    url_parser::UrlParser,
};

use crate::repository::Repository;

use std::{env, path::Path};

pub fn get_work_dir() -> String {
    env::var("WORK_DIR").expect("Missing WORK_DIR environment variable")
}

pub fn load_workspace() -> Vec<Repository> {
    repository::search(Path::new(&get_work_dir()), &UrlParser::default())
}
