//! Functions related to interact with a Git VCS.
mod prompt;
mod status;
pub mod submodules;

use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

pub use prompt::prompt;
pub use status::GitStatus;
pub use status::SubmoduleStatus;
pub use status::status;
pub use submodules::SubmoduleInfo;
use which::which;

/// Get the remote URL of the repository to use to organize the repository
/// within the repo tree. This would be either the origin remote or the first
/// defined remote.
pub fn get_remote_url_repo(
    repo: &git2::Repository,
) -> Result<(PathBuf, Option<String>), git2::Error> {
    Ok((
        repo.path().join("config"),
        repo.find_remote("origin")
            .map_or(
                match repo.remotes()?.get(0) {
                    Some(name) => Some(repo.find_remote(name)?),
                    None => None,
                },
                Some,
            )
            .and_then(|r| r.url().map(String::from)),
    ))
}

/// Start a new git command line.
fn new_git_command() -> Command {
    Command::new(which("git").expect("'git' not found"))
}

/// Get the remote URL of the repository to use to organize the repository
/// within the repo tree. This would be either the origin remote or the first
/// defined remote.
pub fn get_remote_url<P: AsRef<Path>>(
    repo_path: P,
) -> Result<(PathBuf, Option<String>), git2::Error> {
    git2::Repository::discover(repo_path).and_then(|r| get_remote_url_repo(&r))
}

/// Clone a Git repository.
pub fn clone<P: AsRef<OsStr>>(remote_url: &str, location: P) -> i32 {
    let mut res = new_git_command()
        .arg("clone")
        .arg(remote_url)
        .arg(&location)
        .status()
        .expect("Error executing command")
        .code()
        .unwrap();

    if res == 0 {
        res = new_git_command()
            .arg("-C")
            .arg(location)
            .arg("submodule")
            .arg("update")
            .arg("--recursive")
            .arg("--init")
            .status()
            .expect("Error executing command")
            .code()
            .unwrap();
    }

    res
}

/// Fetch a Git repository.
pub fn fetch<P: AsRef<OsStr>>(location: P, quiet: bool) -> i32 {
    let mut cmd = new_git_command();

    cmd.arg("-C")
        .arg(location)
        .arg("fetch")
        .arg("--prune-tags")
        .arg("--force");

    if quiet {
        cmd.arg("--quiet");
    }

    cmd.status()
        .expect("Error executing command")
        .code()
        .unwrap()
}
