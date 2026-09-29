//! The repo-tree configuration.

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::builder::StyledStr;
use clap_complete::engine::CompletionCandidate;
use serde::Deserialize;
use serde::Serialize;

use super::command::CommandConfig;
use super::config_dir;
use super::host::RemoteHost;
use super::host::RemoteHosts;
use super::host::UnknownHost;
use super::host::default_remote_hosts;
use super::prompt::PromptConfig;
use super::repository_location::RepositoryLocation;
use super::tree_space::TreeSpaceConfig;
use crate::error::ConfigError;

/// Obtain a default value for the repo tree root.
fn default_root() -> PathBuf {
    let repo_tree_dir = PathBuf::from(&env::var("REPO_TREE_DIR").expect(
        "Missing \"root\" in configuration file, and REPO_TREE_DIR \
         environment variable",
    ));

    assert!(
        repo_tree_dir.is_absolute(),
        "REPO_TREE_DIR environment variable value must be an absolute path"
    );

    repo_tree_dir
}

/// Configuration of the rt executable.
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// Path the root of the repo tree. Default value obtained through
    /// the environment variable REPO_TREE_DIR.
    #[serde(default = "default_root")]
    pub root: PathBuf,
    /// Configuration related to the hosts we know how to organize repositories
    /// which host there remote.
    #[serde(default = "default_remote_hosts", rename = "host")]
    pub remote_hosts: RemoteHosts,
    /// Configuration for the tree-spaces
    #[serde(default)]
    pub tree: TreeSpaceConfig,
    /// Configuration when having to handle an unknown host (unknown from the
    /// configuration).
    #[serde(default)]
    pub unknown_host: UnknownHost,
    /// Configuration to customize the prompt.
    #[serde(default)]
    pub prompt: PromptConfig,
    /// Configuration regarding allowed repository location outside the repo
    /// tree.
    #[serde(default)]
    pub repository: RepositoryLocation,
    /// Configuration for the different rt sub-commands.
    #[serde(default)]
    pub command: CommandConfig,
}

impl Config {
    /// Internal loading of the configuration, from a configuration content.
    pub(super) fn load_internal(content: &str) -> Result<Self, Box<dyn Error>> {
        let mut ret: Config = toml::from_str(content)?;

        if !ret.root.is_absolute() {
            return Err(Box::new(ConfigError(
                "\"root\" value in configuration file must be an absolute path"
                    .to_string(),
            )));
        }

        // Fill the default remote host configuration if not overridden.
        for (url, host) in default_remote_hosts() {
            if ret.remote_hosts.contains_key(&url) {
                continue;
            }
            ret.remote_hosts.entry(url).or_insert(host);
        }

        Ok(ret)
    }

    /// Load the configuration.
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = config_dir()?.join("config.toml");

        Ok(if config_path.is_file() {
            Self::load_internal(&fs::read_to_string(&config_path)?)?
        } else {
            Self::load_internal("")?
        })
    }

    /// Obtain completion candidates for a CLI host argument.
    pub fn host_completer(&self, current: &OsStr) -> Vec<CompletionCandidate> {
        self.remote_hosts
            .iter()
            .filter(|(host, _)| {
                host.starts_with(current.to_str().unwrap_or(""))
            })
            .map(|(host, data)| {
                CompletionCandidate::new(data.category.name.clone())
                    .help(Some(StyledStr::from(host)))
            })
            .collect()
    }

    /// Get the specified RemoteHost struct for a given host.
    pub fn get_remote_host(&self, host: &str) -> Option<&RemoteHost> {
        self.remote_hosts.get(host)
    }

    /// Find out if the specified path is to be ignored regarding the
    /// configuration.
    pub fn should_be_ignored(&self, path: &Path) -> bool {
        !path.starts_with(&self.root) && self.repository.should_be_ignored(path)
    }
}

#[cfg(test)]
mod test {
    use super::super::host::HostInfo;
    use super::super::tree_category::TreeCategory;
    use super::*;
    use crate::colors::Color;
    use crate::colors::ColoredText;

    impl Config {
        /// Generate a default configuration for tests purposes.
        pub fn test_default() -> Self {
            let mut remote_hosts = default_remote_hosts();
            remote_hosts.insert(
                "test.com".to_string(),
                RemoteHost {
                    category: TreeCategory::new(
                        "test".to_string(),
                        None,
                        ColoredText::new("󰙨", Color::yellow()),
                    ),
                    info: HostInfo { forge: None },
                },
            );
            Self {
                root: PathBuf::from("/home/user/work"),
                remote_hosts,
                tree: TreeSpaceConfig::default(),
                unknown_host: UnknownHost::default(),
                prompt: PromptConfig::default(),
                repository: RepositoryLocation::default(),
                command: CommandConfig::default(),
            }
        }
    }
}
