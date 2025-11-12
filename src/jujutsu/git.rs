//! Function to interact with a Jujutsu which uses a git backend.
use std::{
    error::Error,
    ffi::OsStr,
    fs::{canonicalize, read_to_string},
    path::{Path, PathBuf},
    process::Command,
};

use which::which;

use super::get_repo_dir;
use crate::git;

pub fn get_git_backend_path<P: AsRef<Path>>(
    repo_path: P,
) -> Result<PathBuf, Box<dyn Error>> {
    let store_dir = get_repo_dir(repo_path)?.join("store");

    Ok(canonicalize(
        store_dir.join(read_to_string(store_dir.join("git_target"))?),
    )?)
}

pub fn get_git_backend_repo<P: AsRef<Path>>(
    repo_path: P,
) -> Result<git2::Repository, Box<dyn Error>> {
    Ok(git2::Repository::open(get_git_backend_path(repo_path)?)?)
}

pub fn get_remote_url<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Option<String>, Box<dyn Error>> {
    Ok(git::get_remote_url_repo(&get_git_backend_repo(repo_path)?)?)
}

fn new_jj_command() -> Command {
    Command::new(which("jj").expect("Jujutsu not installed"))
}

pub fn clone<P: AsRef<OsStr>>(remote_url: &str, location: P) -> i32 {
    new_jj_command()
        .arg("git")
        .arg("clone")
        .arg("--no-colocate")
        .arg(remote_url)
        .arg(location)
        .status()
        .expect("Error executing command")
        .code()
        .unwrap()
}

pub fn init_colocate<P: AsRef<OsStr>>(location: P) -> i32 {
    new_jj_command()
        .arg("git")
        .arg("init")
        .arg("--colocate")
        .arg(location)
        .status()
        .expect("Error executing command")
        .code()
        .unwrap()
}

pub fn fetch<P: AsRef<OsStr>>(location: P, quiet: bool) -> i32 {
    let mut cmd = new_jj_command();
    cmd.arg("--repository")
        .arg(location)
        .arg("git")
        .arg("fetch")
        .arg("--all-remotes");

    if quiet {
        cmd.arg("--quiet");
    }

    cmd.status()
        .expect("Error executing command")
        .code()
        .unwrap()
}
