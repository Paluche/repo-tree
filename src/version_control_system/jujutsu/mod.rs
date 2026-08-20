//! Module for retrieving JuJutsu information.
mod git;
mod prompt;
mod repo_state;
use async_trait::async_trait;
mod revsets;
use std::error::Error;
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

pub use git::init_colocate;
use jj_lib::config::StackedConfig;
use jj_lib::local_working_copy::LocalWorkingCopy;
use jj_lib::ref_name::WorkspaceNameBuf;
use jj_lib::repo::ReadonlyRepo;
use jj_lib::repo::RepoLoader;
use jj_lib::repo::StoreFactories;
use jj_lib::settings::UserSettings;
use jj_lib::working_copy::WorkingCopy;

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

/// Get the path to the working copy directory defining its state.
pub fn get_state_path(jj_dir: &Path) -> PathBuf {
    jj_dir.join("working_copy")
}

/// Load an existing jj repository.
pub async fn load(
    repo_path: &Path,
) -> Result<(Arc<ReadonlyRepo>, WorkspaceNameBuf), Box<dyn Error>> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = StoreFactories::default();
    let jj_dir = get_jj_dir(repo_path);

    let loader = RepoLoader::init_from_file_system(
        &user_settings,
        &get_repo_dir(&jj_dir)?,
        &store_factories,
    )?;

    let local_working_copy = LocalWorkingCopy::load(
        loader.store().clone(),
        repo_path.to_path_buf(),
        get_state_path(&jj_dir),
        &user_settings,
    )?;

    Ok((
        loader.load_at_head().await?,
        local_working_copy.workspace_name().to_owned(),
    ))
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

#[async_trait(?Send)]
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

    async fn prompt(&self, config: &Config, prompt: &mut Prompt<'_>) -> i32 {
        let ret =
            super::git::prompt::prompt(config, prompt, &self.repo_path, true);
        if ret != 0 {
            return ret;
        }
        prompt::prompt(config, prompt, &self.repo_path).await
    }

    async fn get_repo_state(&self) -> Result<RepoState, Box<dyn Error>> {
        repo_state::get_repo_state(&self.repo_path).await
    }
}
