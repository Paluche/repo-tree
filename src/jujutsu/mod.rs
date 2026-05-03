//! Module for retrieving JuJutsu information.
pub mod git;
mod prompt;
mod repo_state;
mod revsets;

use std::error::Error;
use std::fs::read_to_string;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

pub use git::get_remote_url;
use jj_lib::config::StackedConfig;
use jj_lib::local_working_copy::LocalWorkingCopy;
use jj_lib::ref_name::WorkspaceNameBuf;
use jj_lib::repo::ReadonlyRepo;
use jj_lib::repo::RepoLoader;
use jj_lib::repo::StoreFactories;
use jj_lib::settings::UserSettings;
use jj_lib::working_copy::WorkingCopy;
pub use prompt::prompt;
pub use repo_state::get_repo_state;

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
