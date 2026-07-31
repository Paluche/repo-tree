//! Format of the configuration file.
//! Should be located in `${XDG_CONFIG_HOME}/repo-tree/config.toml`.
//! If `XDG_CONFIG_HOME` is not set, then we will use the value
//! `${HOME}/.config` in place.
//!
//! See repository README for more information.

mod command;
#[allow(clippy::module_inception)]
mod config;
mod host;
mod identity;
mod prompt;
mod repository_location;
mod tree_category;
mod tree_space;

use std::error::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use clap_complete::engine::CompletionCandidate;
pub use config::Config;
pub use host::RemoteHost;
pub use host::UnknownHost;
pub use identity::Identity;
pub use prompt::JujutsuBookmarkConfig;
pub use prompt::JujutsuPromptConfig;
pub use tree_category::TreeCategory;

/// Obtain the auto-completion candidates for a host argument.
pub fn list_host_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    Config::load().map_or(Vec::new(), |c| c.host_completer(current))
}

/// Path to the repo-tree configuration directory.
pub fn config_dir() -> Result<PathBuf, Box<dyn Error>> {
    Ok(std::env::var("XDG_CONFIG_HOME")
        .map_or(
            std::env::var("HOME").map(|x| Path::new(&x).join(".config")),
            |x| Ok(PathBuf::from(x)),
        )?
        .join("repo-tree"))
}
