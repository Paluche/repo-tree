//! Module for retrieving JuJutsu information.
//! Note: The implementation of the different functions aims at using the jj_lib
//! crate and avoiding directly calling the jj process. While some operation
//! might be simplier done using directly the jj command line. The goal here is
//! to get familiar with the jj_lib, which is supposed to be the interact
//! library to use when we implement a tool that interacts with a jj repository.
//! Although my experiment is that this is not obvious and, this experiment
//! could lead to some improvements on jj-lib crate side.
pub mod git;
mod prompt;
mod repo_state;
mod revsets;

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
pub use repo_state::get_repo_state;

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
    let loader = RepoLoader::init_from_file_system(
        &user_settings,
        &get_repo_dir(repo_path)?,
        &store_factories,
    )?;
    Ok(loader.load_at_head().await?)
}
