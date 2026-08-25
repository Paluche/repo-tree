//! Tools around parsing of repositories URL.
use std::error::Error;
use std::fmt::Display;
use std::path::Path;

use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

use crate::colors::ColoredText;
use crate::config::Config;
use crate::config::RemoteHost;
use crate::error::ParseUrlError;
use crate::error::UnknownRemoteHostError;
use crate::tree::TreeSpace;

/// Parse the remote URL, to capture the different parts.
fn capture_url<'b>(url: &'b str) -> Result<regex::Captures<'b>, ParseUrlError> {
    // scheme-based URLs, e.g.:
    //   https://github.com/owner/repo.git
    //   https://oauth2:<token>@github.com/owner/repo.git
    //   ssh://user@host:2222/owner/repo.git
    //   git://host/owner/repo
    //   file:///path/to/repo.git
    // Captures: scheme, user (optional), host, port (optional), path.
    let re_scheme = Regex::new(concat!(
        r"^(?P<scheme>(?:git|ssh|https?|git\+ssh|rsync|file))",
        r"://(?:(?P<user>[^@]+)@)?(?P<host>[^/:]+)",
        r":?(?:(?P<port>\d+))?/(~[^/]+/)?(?P<path>[^ \r\n]+?)(?:\.git)?/?$"
    ))
    .unwrap();

    // scp-like syntax, e.g.:
    //   git@github.com:owner/repo.git
    //   user@host:/absolute/path/to/repo.git
    // Captures: user (optional), host, path.
    let re_scp = Regex::new(
        r"^(?:(?P<user>[^@:\s]+)@)?(?P<host>[^:\s]+):(?P<path>[^ \r\n]+?)(?:\.git)?/?$"
    ).unwrap();

    re_scheme
        .captures(url)
        .or(re_scp.captures(url))
        .ok_or(ParseUrlError(url.to_string()))
}

/// Representation of the URL of a remote.
#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize)]
pub struct Remote {
    /// URL of the remote.
    pub url: String,

    /// The host part of the URL of the remote.
    pub host_url: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize)]
/// Repository Identifier.
pub struct RepoId {
    /// Information about the host associated with the repository.
    pub remote: Option<Remote>,
    /// Name of the repository.
    pub name: String,
}

impl RepoId {
    /// Create a new repository ID for a Git repository with the provided remote
    /// URL.
    pub fn from_remote_url(remote_url: &str) -> Result<RepoId, ParseUrlError> {
        let remote_cap = capture_url(remote_url)?;
        let host_url = &remote_cap["host"];
        let name = remote_cap["path"].to_string();

        Ok(Self {
            remote: Some(Remote {
                url: remote_url.to_string(),
                host_url: host_url.to_string(),
            }),
            name,
        })
    }

    /// Parse the provided repository remote URL into a host (as Remote struct)
    /// and the local path the repository should be located at in the repo
    /// tree based according to the URL.
    /// This version (in regard to parse_url()) defaults to the local host
    /// location configuration if the remote_url argument is None.
    pub fn from_repo<P: AsRef<Path>>(
        repo_path: &P,
        remote_url: Option<&String>,
    ) -> Result<RepoId, ParseUrlError> {
        if let Some(remote_url) = remote_url {
            Self::from_remote_url(remote_url)
        } else {
            Ok(Self {
                remote: None,
                name: repo_path
                    .as_ref()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            })
        }
    }

    /// Get the host tree category associated with the repository.
    pub fn remote_host<'config>(
        &self,
        config: &'config Config,
    ) -> Result<Option<&'config RemoteHost>, UnknownRemoteHostError> {
        match &self.remote {
            Some(remote) => config
                .get_remote_host(&remote.host_url)
                .ok_or(UnknownRemoteHostError(remote.host_url.to_string()))
                .map(Some),
            None => Ok(None),
        }
    }

    /// Get the host representation associated with that repository. This method
    /// is deterministic compared to getting the category using
    /// host_category().
    pub fn remote_host_repr<'config>(
        &self,
        config: &'config Config,
    ) -> Option<&'config ColoredText> {
        self.remote_host(config)
            .map_or(Some(&config.unknown_host.repr), |r| {
                r.map(|r| &r.category.repr)
            })
    }

    /// Get the host name associated with that repository. This method is
    /// deterministic compared to getting the category using
    /// host_category().
    pub fn remote_host_name<'config>(
        &self,
        config: &'config Config,
    ) -> Option<&'config str> {
        self.remote_host(config)
            .map_or(Some("unknown"), |c| c.map(|c| c.category.name.as_str()))
    }

    /// In which tree the repository should be, based on its ID.
    pub fn expected_tree(&self) -> TreeSpace {
        if self.remote.is_some() {
            TreeSpace::Dev
        } else {
            TreeSpace::Local
        }
    }

    /// Find out if a repository exists only locally (no remote configured).
    pub fn is_local(&self) -> bool {
        self.remote.is_none()
    }

    /// Find out if the repository is archived on the forge it is hosted on.
    #[allow(dead_code)]
    pub async fn is_archived(
        &self,
        config: &Config,
    ) -> Result<bool, Box<dyn Error>> {
        if let Some(remote) = &self.remote {
            if let Some(remote_host) = config.get_remote_host(&remote.host_url)
            {
                remote_host.info.forge.api().is_archived(self).await
            } else {
                Err(Box::new(UnknownRemoteHostError(remote.host_url.clone())))
            }
        } else {
            Ok(false)
        }
    }

    /// Obtain a struct implementing the display for the RepoId.
    pub fn display<'repo_id, 'config>(
        &'repo_id self,
        config: &'config Config,
    ) -> RepoIdDisplay<'repo_id, 'config> {
        RepoIdDisplay {
            repo_id: self,
            remote_host_name: self.remote_host_name(config),
        }
    }
}

/// Struct to display a RepoId.
pub struct RepoIdDisplay<'repo_id, 'config> {
    /// RepoId to display.
    repo_id: &'repo_id RepoId,
    /// Host category data of the RepoId.
    remote_host_name: Option<&'config str>,
}

impl<'repo_id, 'config> Display for RepoIdDisplay<'repo_id, 'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.remote_host_name {
            write!(f, "{name} ")?;
        }
        write!(f, "{}", self.repo_id.name)?;
        if let Some(remote) = &self.repo_id.remote {
            write!(f, " {}", remote.url)?;
        }

        Ok(())
    }
}
