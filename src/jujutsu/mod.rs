//! Module for retrieving JuJutsu information.
pub mod git;
mod prompt;

use std::{
    error::Error,
    fs::read_to_string,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

pub use git::get_remote_url;
use jj_lib::{
    config::StackedConfig,
    repo::{ReadonlyRepo, RepoLoader, StoreFactories},
    settings::UserSettings,
};
pub use prompt::prompt;

pub fn get_repo_dir<P: AsRef<Path>>(repo_path: P) -> io::Result<PathBuf> {
    let jj_dir = repo_path.as_ref().to_path_buf().join(".jj");
    let repo_dir = jj_dir.join("repo");

    Ok(if repo_dir.is_file() {
        // jj workspace
        jj_dir.join(read_to_string(repo_dir)?).canonicalize()?
    } else {
        repo_dir
    })
}

/// Load an existing jj repository.
pub async fn load<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Arc<ReadonlyRepo>, Box<dyn Error>> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = StoreFactories::default();

    // Use RepoLoader to open an existing repo
    let loader = RepoLoader::init_from_file_system(
        &user_settings,
        &get_repo_dir(repo_path)?,
        &store_factories,
    )?;

    // This gives you a loader. You can then load the repo at head:
    Ok(loader.load_at_head().await?)
}
