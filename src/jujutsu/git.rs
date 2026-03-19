//! Function to interact with a Jujutsu repository which uses a git backend.
use std::error::Error;
use std::ffi::OsStr;
use std::fs::canonicalize;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use which::which;

use super::get_repo_dir;
use crate::git;

/// Get the path to the git backend repository.
pub fn get_git_backend_repo<P: AsRef<Path>>(
    repo_path: P,
) -> Result<git2::Repository, Box<dyn Error>> {
    let store_dir = get_repo_dir(repo_path)?.join("store");

    Ok(git2::Repository::open(canonicalize(
        store_dir.join(read_to_string(store_dir.join("git_target"))?),
    )?)?)
}

/// Get the remote URL of the repository.
pub fn get_remote_url<P: AsRef<Path>>(
    repo_path: P,
) -> Result<(PathBuf, Option<String>), Box<dyn Error>> {
    Ok(git::get_remote_url_repo(&get_git_backend_repo(repo_path)?)?)
}

/// Start a new command line to call jj.
fn new_jj_command() -> Command {
    Command::new(which("jj").expect("Jujutsu not installed"))
}

/// Clone a Jujutsu repository.
pub fn clone<P: AsRef<OsStr>>(
    remote_url: &str,
    location: P,
    colocated: bool,
) -> i32 {
    new_jj_command()
        .arg("git")
        .arg("clone")
        .arg(if colocated {
            "--colocate"
        } else {
            "--no-colocate"
        })
        .arg(remote_url)
        .arg(location)
        .status()
        .expect("Error executing command")
        .code()
        .unwrap()
}

/// Initialize a Git-colocated Jujutsu repository.
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

/// Fetch the repository.
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
