//! Module for retrieving JuJutsu information.
pub mod git;
mod prompt;

pub use git::get_remote_url;
use jj_lib::{
    config::StackedConfig,
    repo::{ReadonlyRepo, RepoLoader, StoreFactories},
    settings::UserSettings,
};
pub use prompt::prompt;
use std::sync::Arc;
use std::{error::Error, path::Path};

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
