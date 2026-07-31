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
#[cfg(test)]
mod test;
mod tree_category;
mod tree_space;
mod ui;

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

pub use command::GitStatusCommandConfig;
pub use config::Config;
pub use host::RemoteHost;
#[cfg(test)]
pub use identity::NoSigning;
pub use identity::ResolvedIdentity;
pub use identity::SignOn;
pub use identity::Signing;
pub use identity::SigningBackend;
#[cfg(test)]
pub use identity::SigningKey;
#[cfg(test)]
pub use identity::Tokens;
pub use prompt::GitSummarizeStatusConfig;
pub use prompt::GitSummarizeSubmoduleStatusConfig;
pub use prompt::JujutsuBookmarkConfig;
pub use prompt::JujutsuTagConfig;
pub use tree_category::TreeCategory;

use crate::error::ConfigError;

/// Path to the repo-tree configuration directory.
pub fn config_dir() -> Result<PathBuf, Box<dyn Error>> {
    config_dir_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

/// Path to the repo-tree configuration directory from the values of the
/// `XDG_CONFIG_HOME` and `HOME` environment variables.
///
/// As required by the XDG Base Directory specification, an empty or relative
/// `XDG_CONFIG_HOME` is ignored.
fn config_dir_from(
    xdg_config_home: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, Box<dyn Error>> {
    let base = match xdg_config_home.map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => dir,
        _ => Path::new(&home.filter(|home| !home.is_empty()).ok_or_else(
            || {
                ConfigError(
                    "cannot locate the configuration file, neither \
                     XDG_CONFIG_HOME nor HOME environment variables are set"
                        .to_string(),
                )
            },
        )?)
        .join(".config"),
    };

    Ok(base.join("repo-tree"))
}

/// Path to the repo-tree configuration file.
pub fn config_file() -> Result<PathBuf, Box<dyn Error>> {
    Ok(config_dir()?.join("config.toml"))
}
