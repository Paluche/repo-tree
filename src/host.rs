//! Different type of hosts.
use std::fmt::Display;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::config::HostInfo;
use crate::config::LocalHost;
use crate::config::RemoteHost;
use crate::config::UnknownHost;
use crate::error::UnknownRemoteHostError;

#[derive(Clone, Hash, PartialEq, Serialize, Deserialize)]
/// The different type of host one repository can be associated with.
pub enum Remote {
    /// Remote of the repository associated with the repository.
    Remote(String, String),
    /// Repository exists only locally.
    Local,
}

impl Remote {
    /// Get the remote URL, if the host is remote.
    pub fn url(&self) -> Option<&String> {
        match self {
            Self::Remote(url, _host) => Some(url),
            Self::Local => None,
        }
    }

    /// Resolve the host based on the configuration.
    pub fn host<'config>(&self, config: &'config Config) -> Host<'config> {
        match self {
            Self::Remote(_url, host) => config.get_remote_host(host).map_or(
                Host::UnknownRemote(host.to_string(), &config.unknown_host),
                Host::Remote,
            ),
            Self::Local => Host::Local(&config.local),
        }
    }

    /// Find out if the repository is hosted locally.
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }
}

#[derive(Clone, Hash)]
/// The different type of host one repository can be associated with.
pub enum Host<'config> {
    /// Repository is associated with a remote repository stored on the linked
    /// host.
    Remote(&'config RemoteHost),
    /// Repository is associated with a remote repository stored on an unknown
    /// host for which we are missing the associated configuration.
    UnknownRemote(String, &'config UnknownHost),
    /// Repository exists only locally.
    Local(&'config LocalHost),
}

impl<'config> Host<'config> {
    /// Name of the remote host.
    pub fn name(&self) -> Result<&String, UnknownRemoteHostError> {
        match self {
            Self::Remote(remote_host) => Ok(&remote_host.name),
            Self::UnknownRemote(host_url, _) => {
                Err(UnknownRemoteHostError(host_url.to_owned()))
            }
            Self::Local(local_host) => Ok(&local_host.name),
        }
    }

    /// Name of the directory for that host in the repo tree.
    pub fn dir_name(&self) -> Result<String, UnknownRemoteHostError> {
        match self {
            Self::Remote(remote_host) => Ok(remote_host.dir_name()),
            Self::UnknownRemote(host_url, _) => {
                Err(UnknownRemoteHostError(host_url.to_owned()))
            }
            Self::Local(local_host) => Ok(local_host.dir_name()),
        }
    }

    /// Get the full path to the directory for that host.
    pub fn dir_path(
        &self,
        config: &Config,
    ) -> Result<PathBuf, UnknownRemoteHostError> {
        self.dir_name().map(|d| config.repo_tree_dir.join(d))
    }

    /// Get the short representation of the host.
    pub fn repr<'host>(&'host self) -> Box<dyn Display + 'host> {
        match self {
            Self::Remote(remote_host) => Box::new(remote_host),
            Self::UnknownRemote(_, unknown_host) => Box::new(unknown_host),
            Self::Local(local_host) => Box::new(local_host),
        }
    }
}

impl<'config> Display for Host<'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(dir_name) = self.dir_name() {
            write!(f, "{dir_name}")
        } else {
            write!(f, "?????")
        }
    }
}
