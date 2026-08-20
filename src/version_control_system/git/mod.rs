//! Functions related to interact with a Git VCS.
pub mod prompt;
mod status;
pub mod submodules;

use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use async_trait::async_trait;
pub use status::GitStatus;
pub use status::SubmoduleStatus;
pub use status::status;
pub use submodules::SubmoduleInfo;
use which::which;

use super::VcsRepository;
use crate::config::Config;
use crate::error::NotImplementedError;
use crate::prompt::Prompt;
use crate::repo_state::RepoState;

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
                match repo.remotes()?.get(0)? {
                    Some(name) => Some(repo.find_remote(name)?),
                    None => None,
                },
                Some,
            )
            .and_then(|r| r.url().ok().map(String::from)),
    ))
}

/// Start a new git command line.
fn new_git_command() -> Command {
    Command::new(which("git").expect("'git' not found"))
}

/// Interact with a Git repository.
pub struct GitVcs {
    /// Path to the root of the Git repository.
    repo_path: PathBuf,
}

impl GitVcs {
    /// Create a new GitVcs structure.
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
        }
    }
}

#[async_trait(?Send)]
impl VcsRepository for GitVcs {
    fn get_remote_url(
        &self,
    ) -> Result<(PathBuf, Option<String>), Box<dyn Error>> {
        let (remote_config, remote_url) =
            git2::Repository::discover(&self.repo_path)
                .and_then(|r| get_remote_url_repo(&r))?;

        Ok((remote_config, remote_url))
    }

    fn clone(&self, remote_url: &str) -> i32 {
        let mut res = new_git_command()
            .arg("clone")
            .arg(remote_url)
            .arg(&self.repo_path)
            .status()
            .expect("Error executing command")
            .code()
            .unwrap();

        if res == 0 {
            res = new_git_command()
                .arg("-C")
                .arg(&self.repo_path)
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

    fn fetch(&self, quiet: bool) -> i32 {
        let mut cmd = new_git_command();

        cmd.arg("-C")
            .arg(&self.repo_path)
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

    async fn prompt(&self, config: &Config, prompt: &mut Prompt<'_>) -> i32 {
        prompt::prompt(config, prompt, &self.repo_path, false)
    }

    async fn get_repo_state(&self) -> Result<RepoState, Box<dyn Error>> {
        Err(Box::new(NotImplementedError(
            "Repository state for Git Version Control System".to_string(),
        )))
    }
}
