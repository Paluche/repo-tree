mod prompt;
mod status;
pub mod submodules;

pub use prompt::prompt;
pub use status::{GitStatus, SubmoduleStatus, status};
use std::{ffi::OsStr, path::Path, process::Command};
pub use submodules::SubmoduleInfo;
use which::which;

pub fn get_remote_url_repo(
    repo: &git2::Repository,
) -> Result<Option<String>, git2::Error> {
    Ok(repo
        .find_remote("origin")
        .map_or(
            match repo.remotes()?.get(0) {
                Some(name) => Some(repo.find_remote(name)?),
                None => None,
            },
            Some,
        )
        .and_then(|r| r.url().map(String::from)))
}

fn new_git_command() -> Command {
    Command::new(which("git").expect("'git' not found"))
}

pub fn get_remote_url<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Option<String>, git2::Error> {
    let repo = git2::Repository::discover(repo_path)?;

    get_remote_url_repo(&repo)
}

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

pub fn fetch<P: AsRef<OsStr>>(location: P) -> i32 {
    new_git_command()
        .arg("-C")
        .arg(location)
        .arg("fetch")
        .arg("--prune-tags")
        .arg("--force")
        .status()
        .expect("Error executing command")
        .code()
        .unwrap()
}
