//! Module for retrieving JuJutsu information.
mod prompt;

use crate::git;
use git2::Repository;
use jj_lib::config::StackedConfig;
use jj_lib::repo::{ReadonlyRepo, RepoLoader, StoreFactories};
use jj_lib::settings::UserSettings;
use std::sync::Arc;
use std::{
    error::Error,
    path::{Path, PathBuf},
};
pub use prompt::prompt;

pub fn get_remote_url<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Option<String>, Box<dyn Error>> {
    let mut git_dir = PathBuf::new();
    git_dir.push(&repo_path);
    git_dir.push(".jj");
    git_dir.push("repo");
    git_dir.push("store");
    git_dir.push("git");
    let repo = Repository::open(git_dir)?;

    Ok(git::get_remote_url_repo(&repo)?)
}

/// Load an existing jj repository.
pub fn load<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Arc<ReadonlyRepo>, Box<dyn Error>> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = StoreFactories::default();

    // Use RepoLoader to open an existing repo
    let loader = RepoLoader::init_from_file_system(
        &user_settings,
        repo_path.as_ref().join(".jj").join("repo").as_path(),
        &store_factories,
    )?;

    // This gives you a loader. You can then load the repo at head:
    Ok(loader.load_at_head()?)
}
