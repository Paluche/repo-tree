//! Function to interact with a Jujutsu repository which uses a git backend.
use std::error::Error;
use std::ffi::OsStr;
use std::fs::canonicalize;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

use super::command::JujutsuCommand;
use super::get_jj_dir;
use super::get_repo_dir;

/// Get the path to the git backend repository.
pub fn get_git_dir(repo_path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let store_dir = get_repo_dir(&get_jj_dir(repo_path))?.join("store");
    Ok(canonicalize(
        store_dir.join(read_to_string(store_dir.join("git_target"))?),
    )?)
}

/// Get the remote URL of the repository.
pub fn get_remote_url(
    repo_path: &Path,
) -> Result<(PathBuf, Option<String>), Box<dyn Error>> {
    let remotes = get_remotes(repo_path)?;

    Ok((
        get_git_dir(repo_path)?.join("config"),
        remotes
            .iter()
            .find(|(n, _)| n == "origin")
            .or_else(|| remotes.first())
            .map(|(_, u)| u.to_string()),
    ))
}

/// Get the configured remotes of the repository.
pub fn get_remotes(
    repo_path: &Path,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    Ok(JujutsuCommand::repo(repo_path)?
        .arg("git")
        .arg("remote")
        .arg("list")
        .output_lines()?
        .iter()
        .map(|l| {
            let mut parts = l.split(' ');
            (
                parts.next().unwrap().to_string(),
                parts.next().unwrap().to_string(),
            )
        })
        .collect())
}

/// Clone a Jujutsu repository.
pub fn clone<P: AsRef<OsStr>>(
    remote_url: &str,
    location: P,
    colocated: bool,
) -> Result<(), Box<dyn Error>> {
    JujutsuCommand::global()?
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
}

/// Initialize a Git-colocated Jujutsu repository.
pub fn init_colocate<P: AsRef<OsStr>>(
    location: P,
) -> Result<(), Box<dyn Error>> {
    JujutsuCommand::global()?
        .arg("git")
        .arg("init")
        .arg("--colocate")
        .arg(location)
        .status()
}

/// Fetch the repository.
pub fn fetch<P: AsRef<OsStr>>(
    location: P,
    quiet: bool,
) -> Result<(), Box<dyn Error>> {
    let mut cmd = JujutsuCommand::repo(location)?;
    cmd.arg("git").arg("fetch");

    if quiet {
        cmd.arg("--quiet");
    }

    cmd.status()
}
