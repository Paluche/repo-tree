//! Format of the configuration file.
//! Should be located in `${XDG_CONFIG_HOME}/repo-tree/config.toml`.
//! If `XDG_CONFIG_HOME` is not set, then we will use the value
//! `${HOME}/.config` in place.
//!
//! See repository README for more information.

mod command;
#[expect(clippy::module_inception)]
mod config;
mod host;
mod identity;
mod prompt;
mod repository_location;
mod tree_category;
mod tree_space;

use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

pub use config::Config;
pub use host::RemoteHost;
pub use host::UnknownHost;
pub use identity::Identity;
pub use prompt::JujutsuBookmarkConfig;
pub use prompt::JujutsuTagConfig;
pub use tree_category::TreeCategory;

/// Path to the repo-tree configuration directory.
pub fn config_dir() -> Result<PathBuf, Box<dyn Error>> {
    Ok(std::env::var("XDG_CONFIG_HOME")
        .map_or(
            std::env::var("HOME").map(|x| Path::new(&x).join(".config")),
            |x| Ok(PathBuf::from(x)),
        )?
        .join("repo-tree"))
}
