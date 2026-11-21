//! Function to interact with a Jujutsu repository which uses a git backend.
use std::error::Error;
use std::ffi::OsStr;
use std::fs::canonicalize;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

use super::super::VcsRemote;
use super::command::JujutsuCommand;
use super::config;
use super::get_jj_dir;
use super::get_repo_dir;
use crate::config::ResolvedIdentity;

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
            .find(|r| r.name == "origin")
            .or_else(|| remotes.first())
            .map(|r| r.url.clone()),
    ))
}

/// Get the configured remotes of the repository.
pub fn get_remotes(repo_path: &Path) -> Result<Vec<VcsRemote>, Box<dyn Error>> {
    parse_remote_list(
        &JujutsuCommand::repo(repo_path)?
            .arg("git")
            .arg("remote")
            .arg("list")
            .output_lines()?,
    )
}

/// Parse the output of `jj git remote list`, a remote per line, with its name
/// and URL separated by a space.
fn parse_remote_list(
    lines: &[String],
) -> Result<Vec<VcsRemote>, Box<dyn Error>> {
    lines
        .iter()
        .map(|line| {
            let (name, url) = line.split_once(' ').ok_or_else(|| {
                format!(
                    "Unexpected line in jj git remote list output: {line:?}"
                )
            })?;
            Ok(VcsRemote {
                name: name.to_string(),
                url: url.to_string(),
            })
        })
        .collect()
}

/// Clone a Jujutsu repository. The identity, if any, is the configuration
/// of the clone, so that the first working-copy commit already has the
/// right author.
pub fn clone<P: AsRef<OsStr>>(
    remote_url: &str,
    location: P,
    colocated: bool,
    identity: Option<&ResolvedIdentity>,
) -> Result<(), Box<dyn Error>> {
    let mut command = JujutsuCommand::global()?;

    if let Some(identity) = identity {
        for arg in config::config_args(identity) {
            command.arg("--config").arg(arg);
        }
    }

    command
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
pub fn fetch<P: AsRef<Path>>(
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines of a command output.
    fn lines(lines: &[&str]) -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn parse_remotes() {
        assert_eq!(
            parse_remote_list(&lines(&[
                "origin https://example.test/o/r.git",
                "upstream git@example.test:u/r.git",
            ]))
            .expect("valid remotes"),
            [
                VcsRemote {
                    name: "origin".into(),
                    url: "https://example.test/o/r.git".into(),
                },
                VcsRemote {
                    name: "upstream".into(),
                    url: "git@example.test:u/r.git".into(),
                },
            ]
        );
        assert_eq!(parse_remote_list(&[]).expect("no remote"), []);
    }

    #[test]
    fn parse_malformed_remote_fails() {
        let error =
            parse_remote_list(&lines(&["origin"])).expect_err("missing URL");

        assert!(error.to_string().contains("\"origin\""), "{error}");
    }
}
