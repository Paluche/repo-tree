//! Module for retrieving Jujutsu information.
mod bookmark;
mod command;
mod config;
mod git;
mod prompt;
mod repo_state;
mod revset;
mod tag;
mod workspace;

use std::error::Error;
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use std::path::PathBuf;

pub use git::init_colocate;

use super::VcsRemote;
use super::VcsRepository;
use super::VcsWorkspace;
use super::identity::ConfigChange;
use crate::config::Config;
use crate::config::ResolvedIdentity;
use crate::error::NotARepositoryError;
use crate::prompt::Prompt;
use crate::repo_state::RepoState;
use crate::repository::Workspace;

/// Name of the pseudo remote Jujutsu uses to represent the Git repository
/// it is colocated with. Its bookmarks and tags mirror the local ones, so they
/// are ignored.
const GIT_REMOTE: &str = "git";

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
        jj_dir
            .join(read_to_string(repo_dir)?.trim_end_matches('\n'))
            .canonicalize()?
    } else {
        repo_dir
    })
}

/// Interact with a Jujutsu repository.
pub struct JujutsuVcs<'repo_tree> {
    /// Path to the root of the Jujutsu repository.
    workspace: &'repo_tree Workspace,
    /// If the Jujutsu repository is colocated with a Git repository.
    colocated: bool,
}

impl<'repo_tree> JujutsuVcs<'repo_tree> {
    /// Create a new `JujutsuVcs` structure.
    pub fn new(workspace: &'repo_tree Workspace, colocated: bool) -> Self {
        Self {
            workspace,
            colocated,
        }
    }

    /// Open the Git repository backing the Jujutsu repository.
    fn get_git_repository(&self) -> Result<git2::Repository, Box<dyn Error>> {
        Ok(git2::Repository::open(git::get_git_dir(
            self.workspace.path(),
        )?)?)
    }
}

impl VcsRepository for JujutsuVcs<'_> {
    fn get_remote_url(
        &self,
    ) -> Result<(PathBuf, Option<String>), Box<dyn Error>> {
        git::get_remote_url(self.workspace.path())
    }

    fn get_workspace_store(&self) -> Result<Option<PathBuf>, Box<dyn Error>> {
        let index = get_repo_dir(&get_jj_dir(self.workspace.path()))?
            .join("workspace_store")
            .join("index");

        Ok(index.is_file().then_some(index))
    }

    fn get_remotes(&self) -> Result<Vec<VcsRemote>, Box<dyn Error>> {
        git::get_remotes(self.workspace.path())
    }

    fn clone(
        &self,
        remote_url: &str,
        identity: Option<&ResolvedIdentity>,
    ) -> Result<(), Box<dyn Error>> {
        git::clone(
            remote_url,
            self.workspace.path(),
            self.colocated,
            identity,
        )?;

        if let Some(identity) = identity {
            self.apply_config(&self.identity_changes(identity)?)?;
        }

        Ok(())
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

    fn get_workspace(&self) -> Result<Option<VcsWorkspace>, Box<dyn Error>> {
        let workspaces = workspace::list_workspaces(self.workspace.path())?;

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

    fn remove_workspace(&self, config: &Config) -> Result<(), Box<dyn Error>> {
        let Some(workspace) = self.get_workspace()? else {
            return Err(format!(
                "{} is the only workspace of the repository",
                self.workspace.path().display()
            )
            .into());
        };

        if workspace.name == workspace::DEFAULT_WORKSPACE {
            return Err(format!(
                "{} is the default workspace of the repository",
                self.workspace.path().display()
            )
            .into());
        }

        workspace::forget_workspace(self.workspace.path(), &workspace.name)?;

        self.workspace.rm(config)?;

        Ok(())
    }

    fn forget_missing_workspaces(
        &self,
    ) -> Result<Vec<VcsWorkspace>, Box<dyn Error>> {
        let missing: Vec<VcsWorkspace> =
            workspace::list_workspaces(self.workspace.path())?
                .into_iter()
                .filter(|w| {
                    w.name != workspace::DEFAULT_WORKSPACE && !w.path.exists()
                })
                .collect();

        for w in &missing {
            workspace::forget_workspace(self.workspace.path(), &w.name)?;
        }

        Ok(missing)
    }

    fn identity_changes(
        &self,
        identity: &ResolvedIdentity,
    ) -> Result<Vec<ConfigChange>, Box<dyn Error>> {
        let mut changes =
            config::identity_changes(self.workspace.path(), identity)?;

        if self.colocated {
            changes.extend(super::git::config::identity_changes(
                &self.get_git_repository()?,
                identity,
            )?);
        }

        Ok(changes)
    }

    fn apply_config(
        &self,
        changes: &[ConfigChange],
    ) -> Result<(), Box<dyn Error>> {
        let (git, jj): (Vec<ConfigChange>, Vec<ConfigChange>) =
            changes.iter().cloned().partition(|c| c.vcs == "git");

        config::apply(self.workspace.path(), &jj)?;

        if !git.is_empty() {
            super::git::config::apply(&self.get_git_repository()?, &git)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn repo_dir_of_main_workspace() {
        let dir = tempfile::tempdir().expect("cannot create directory");
        let jj_dir = dir.path().join(".jj");
        fs::create_dir_all(jj_dir.join("repo")).expect("cannot create dir");

        assert_eq!(
            get_repo_dir(&jj_dir).expect("main workspace"),
            jj_dir.join("repo")
        );
    }

    #[test]
    fn repo_dir_of_secondary_workspace() {
        let dir = tempfile::tempdir().expect("cannot create directory");
        let main_repo = dir.path().join("main/.jj/repo");
        fs::create_dir_all(&main_repo).expect("cannot create dir");
        let jj_dir = dir.path().join("secondary/.jj");
        fs::create_dir_all(&jj_dir).expect("cannot create dir");
        // jj stores the path relative to the .jj directory.
        fs::write(jj_dir.join("repo"), "../../main/.jj/repo\n")
            .expect("cannot create file");

        assert_eq!(
            get_repo_dir(&jj_dir).expect("secondary workspace"),
            main_repo.canonicalize().expect("canonical path")
        );
    }

    #[test]
    fn repo_dir_of_broken_secondary_workspace_fails() {
        let dir = tempfile::tempdir().expect("cannot create directory");
        let jj_dir = dir.path().join(".jj");
        fs::create_dir_all(&jj_dir).expect("cannot create dir");
        fs::write(jj_dir.join("repo"), "../missing").expect("cannot create");

        assert!(get_repo_dir(&jj_dir).is_err());
    }
}
