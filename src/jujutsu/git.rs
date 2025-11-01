//! Function to interact with a Jujutsu which uses a git backend.
//!
use std::{
    error::Error,
    fs::{canonicalize, read_to_string},
    path::{Path, PathBuf},
};

use crate::git;

pub fn get_git_backend_repo<P: AsRef<Path>>(
    repo_path: P,
) -> Result<git2::Repository, Box<dyn Error>> {
    let mut store_dir = PathBuf::new();
    store_dir.push(&repo_path);
    store_dir.push(".jj");
    store_dir.push("repo");
    store_dir.push("store");

    Ok(git2::Repository::open(canonicalize(
        store_dir.join(read_to_string(store_dir.join("git_target"))?),
    )?)?)
}

pub fn get_remote_url<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Option<String>, Box<dyn Error>> {
    Ok(git::get_remote_url_repo(&get_git_backend_repo(repo_path)?)?)
}
