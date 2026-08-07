mod bookmark;
mod config;
mod git;
mod prompt;
mod repo_state;
mod revset;

use std::error::Error;
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub use git::init_colocate;

use super::VcsRepository;
use crate::config::Config;
use crate::prompt::Prompt;
use crate::repo_state::RepoState;

/// Get the path to the jj directory from the repository root path.
pub fn get_jj_dir(repo_path: &Path) -> PathBuf {
    repo_path.to_path_buf().join(".jj")
}

/// Get path to the jj repository, supporting the fact that the original
/// repository is potentially a workspace.
pub fn get_repo_dir(jj_dir: &Path) -> io::Result<PathBuf> {
    let repo_dir = jj_dir.join("repo");

    Ok(if repo_dir.is_file() {
        // jj workspace.
        jj_dir.join(read_to_string(repo_dir)?).canonicalize()?
    } else {
        repo_dir
    })
}

/// Interact with a JuJutsu repository.
pub struct JujutsuVcs {
    /// Path to the root of the JuJutsu repository.
    repo_path: PathBuf,
    /// If the Jujutsu repository is colocated with a Git repository.
    colocated: bool,
}

impl JujutsuVcs {
    /// Create a new JujutsuVcs structure.
    pub fn new(repo_path: &Path, colocated: bool) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
            colocated,
        }
    }
}

impl VcsRepository for JujutsuVcs {
    fn get_remote_url(
        &self,
    ) -> Result<(PathBuf, Option<String>), Box<dyn Error>> {
        git::get_remote_url(&self.repo_path)
    }

    fn clone(&self, remote_url: &str) -> i32 {
        git::clone(remote_url, &self.repo_path, self.colocated)
    }

    fn fetch(&self, quiet: bool) -> i32 {
        git::fetch(&self.repo_path, quiet)
    }

    fn prompt(&self, config: &Config, prompt: &mut Prompt<'_>) -> i32 {
        let ret =
            super::git::prompt::prompt(config, prompt, &self.repo_path, true);
        if ret != 0 {
            return ret;
        }
        prompt::prompt(config, prompt, &self.repo_path)
    }

    fn get_repo_state(&self) -> Result<RepoState, Box<dyn Error>> {
        repo_state::get_repo_state(&self.repo_path)
    }

    async fn get_workspace_name(&self) -> Result<String, Box<dyn Error>> {
        Ok("default".to_string())
    }

    async fn create_workspace(
        &self,
        name: &str,
        destination: &Path,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
