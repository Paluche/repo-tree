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
    prompt_action::prompt, resolve_action::resolve, status_action::status,
    url_parser::UrlParser,
};

use crate::repository::Repository;

use std::{env, path::Path};

pub fn get_work_dir() -> String {
    env::var("WORK_DIR").expect("Missing WORK_DIR environment variable")
}

pub fn load_workspace() -> Vec<Repository> {
    let work_dir =
        env::var("WORK_DIR").expect("Missing WORK_DIR environment variable");
    let work_dir = Path::new(&work_dir);
    let url_parser = UrlParser::default();
    repository::search(work_dir, &url_parser)
}
