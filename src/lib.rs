//! Libraries for the repo-tools utils
//!
mod git;
mod prompt_action;
mod repository;
mod resolve_action;
mod status_action;
mod url_parsing;
mod version_control_system;

pub use crate::{
    prompt_action::prompt, resolve_action::resolve, status_action::status,
};

use crate::repository::Repository;

use std::{env, path::Path};

pub fn load_workspace() -> Vec<Repository> {
    let work_dir =
        env::var("WORK_DIR").expect("Missing WORK_DIR environment variable");
    let work_dir = Path::new(&work_dir);
    repository::search(work_dir)
}
