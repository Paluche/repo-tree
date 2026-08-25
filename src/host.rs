//! Different type of hosts.
use std::fmt::Display;
use std::path::PathBuf;

use crate::config::Config;
use crate::config::HostInfo;
use crate::config::LocalHost;
use crate::config::RemoteHost;
use crate::config::UnknownHost;
use crate::error::UnknownRemoteHostError;
use crate::forge::Forge;

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
        self.dir_name().map(|d| config.root.join(d))
    }

    /// Get the short representation of the host.
    pub fn repr<'host>(&'host self) -> Box<dyn Display + 'host> {
        match self {
            Self::Remote(remote_host) => Box::new(remote_host),
            Self::UnknownRemote(_, unknown_host) => Box::new(unknown_host),
            Self::Local(local_host) => Box::new(local_host),
        }
    }

    /// Get the forge enum value associated with host.
    #[allow(dead_code)]
    pub fn forge(&self) -> Forge {
        match self {
            Self::Remote(remote_host) => remote_host.forge(),
            Self::UnknownRemote(_, unknown_host) => unknown_host.forge(),
            Self::Local(local_host) => local_host.forge(),
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
