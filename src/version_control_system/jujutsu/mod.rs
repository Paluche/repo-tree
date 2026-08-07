mod bookmark;
mod command;
mod config;
mod git;
mod prompt;
mod repo_state;
mod revset;
mod workspace;

use std::error::Error;
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub use git::init_colocate;

use super::VcsRepository;
use super::VcsWorkspace;
use crate::config::Config;
use crate::error::NotARepositoryError;
use crate::prompt::Prompt;
use crate::repo_state::RepoState;
use crate::repository::Workspace;
use crate::ui::Ui;

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
pub struct JujutsuVcs<'repo_tree> {
    /// Path to the root of the JuJutsu repository.
    workspace: &'repo_tree Workspace,
    /// If the Jujutsu repository is colocated with a Git repository.
    colocated: bool,
}

impl<'repo_tree> JujutsuVcs<'repo_tree> {
    /// Create a new JujutsuVcs structure.
    pub fn new(workspace: &'repo_tree Workspace, colocated: bool) -> Self {
        Self {
            workspace,
            colocated,
        }
    }
}

impl<'repo_tree> VcsRepository for JujutsuVcs<'repo_tree> {
    fn get_remote_url(
        &self,
    ) -> Result<(PathBuf, Option<String>), Box<dyn Error>> {
        git::get_remote_url(self.workspace.path())
    }

    fn clone(&self, remote_url: &str) -> Result<(), Box<dyn Error>> {
        git::clone(remote_url, self.workspace.path(), self.colocated)
    }

    fn fetch(&self, quiet: bool) -> Result<(), Box<dyn Error>> {
        git::fetch(self.workspace.path(), quiet)
    }

    fn prompt(
        &self,
        config: &Config,
        prompt: &mut Prompt<'_>,
    ) -> Result<(), Box<dyn Error>> {
        if let Err(err) = super::git::prompt::prompt(
            config,
            prompt,
            self.workspace.path(),
            true,
        ) && err.downcast_ref::<NotARepositoryError>().is_none()
        {
            return Err(err);
        }
        prompt::prompt(config, prompt, self.workspace.path())
    }

    fn get_repo_state(&self) -> Result<RepoState, Box<dyn Error>> {
        repo_state::get_repo_state(self.workspace.path())
    }

    fn get_workspace(
        &self,
        ui: &Ui<'_>,
        config: &Config,
    ) -> Result<Option<VcsWorkspace>, Box<dyn Error>> {
        let workspaces =
            workspace::list_workspaces(ui, config, self.workspace.path())?;

        if workspaces.len() <= 1 {
            return Ok(None);
        }

        Ok(workspaces
            .into_iter()
            .find(|w| &w.path == self.workspace.path()))
    }

    fn add_workspace(
        &self,
        name: &str,
        destination: &Path,
    ) -> Result<(), Box<dyn Error>> {
        workspace::add_workspace(self.workspace.path(), name, destination)
    }

    fn remove_workspace(&self,
        ui: &Ui<'_>,
        config: &Config,
        ) -> Result<(), Box<dyn Error>> {
        let Some(workspace) = self.get_workspace(ui, config)? else {
            eprintln!("No workspace to remove");
            return Ok(()); // TODO Return an Err
        };

        if workspace.name == "default" {
            eprintln!("Cannot remove default workspace.");
            return Ok(()); // TODO Return an Err
        }

        workspace::forget_workspace(self.workspace.path(), &workspace.name)?;

        self.workspace.rm();

        Ok(())
    }
}
