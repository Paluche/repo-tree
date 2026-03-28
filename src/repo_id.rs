//! Tools around parsing of repositories URL.
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::config::LocalHost;
use crate::config::RemoteHost;
use crate::config::UnknownHost;
use crate::error::ParseUrlError;
use crate::error::UnknownRemoteHostError;

#[derive(Clone, Debug, PartialEq, Hash, Default)]
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
    #[default]
    /// Host not resolved.
    NotResolved,
}

impl<'config> Host<'config> {
    /// Create a new Host::Local enumeration value.
    pub fn local(config: &'config Config) -> Self {
        Self::Local(&config.local)
    }

    /// Create a new Host::UnknownRemote enumeration value.
    pub fn unknown_remote(config: &'config Config, url: String) -> Self {
        Self::UnknownRemote(url, &config.unknown_host)
    }

    /// Find out if the enum value is representing a local host.
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    /// Find out if the enum value is representing a host which remote is
    /// unknown based on the configuration.
    pub fn is_unknown_remote(&self) -> bool {
        matches!(self, Self::UnknownRemote(_, _))
    }

    /// Name of the remote host.
    pub fn name(&self) -> Result<&String, UnknownRemoteHostError> {
        match self {
            Self::Remote(remote_host) => Ok(&remote_host.name),
            Self::UnknownRemote(host_url, _) => {
                Err(UnknownRemoteHostError(host_url.to_owned()))
            }
            Self::Local(local_host) => Ok(&local_host.name),
            Self::NotResolved => panic!("Error in the code"),
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
            Self::NotResolved => panic!("Error in the code"),
        }
    }

    /// Get the full path to the directory for that host.
    pub fn get_host_dir(
        &self,
        config: &Config,
    ) -> Result<PathBuf, UnknownRemoteHostError> {
        self.dir_name().map(|d| config.repo_tree_dir.join(d))
    }

    /// Get the short representation of the host.
    pub fn repr(&self) -> String {
        match self {
            Self::Remote(remote_host) => remote_host.repr(),
            Self::UnknownRemote(_, unknown_host) => unknown_host.repr(),
            Self::Local(local_host) => local_host.repr(),
            Self::NotResolved => panic!("Error in the code"),
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

/// Either the repository is within the ${REPO_TREE_DIR}/local directory
/// allowing the user to organize as see fits this directory.
/// Or take the directory name.
fn compute_local_path<P: AsRef<Path>>(
    repo_tree_dir: &Path,
    repo_path: &P,
) -> String {
    let local_dir = repo_tree_dir.join("local");
    let repo_path = repo_path.as_ref();
    assert!(repo_path.is_absolute(), "repo_path is not absolute");
    assert!(local_dir.is_absolute(), "local_dir is not absolute");

    if repo_path.starts_with(&local_dir) {
        repo_path
            .iter()
            .skip(local_dir.iter().count())
            .collect::<PathBuf>()
            .display()
            .to_string()
    } else {
        repo_path.file_name().unwrap().to_str().unwrap().to_owned()
    }
}

/// Parse the remote URL, to capture the different parts.
fn capture_url<'b>(url: &'b str) -> Result<regex::Captures<'b>, ParseUrlError> {
    // scheme-based URLs, e.g.:
    //   https://github.com/owner/repo.git
    //   https://oauth2:<token>@github.com/owner/repo.git
    //   ssh://user@host:2222/owner/repo.git
    //   git://host/owner/repo
    //   file:///path/to/repo.git
    // Captures: scheme, user (optional), host, port (optional), path
    let re_scheme = Regex::new(concat!(
        r"^(?P<scheme>(?:git|ssh|https?|git\+ssh|rsync|file))",
        r"://(?:(?P<user>[^@]+)@)?(?P<host>[^/:]+)",
        r"(?::(?P<port>\d+))?/(?P<path>[^ \r\n]+?)(?:\.git)?/?$"
    ))
    .unwrap();

    // scp-like syntax, e.g.:
    //   git@github.com:owner/repo.git
    //   user@host:/absolute/path/to/repo.git
    // Captures: user (optional), host, path
    let re_scp = Regex::new(
        r"^(?:(?P<user>[^@:\s]+)@)?(?P<host>[^:\s]+):(?P<path>[^ \r\n]+?)(?:\.git)?/?$"
    ).unwrap();

    // local paths (file:// handled above; this covers bare filesystem paths)
    // matches:
    //   /absolute/path/to/repo.git
    //   ./relative/path
    //   ../relative/path
    //   ~/path
    //   C:\path\to\repo.git
    // let re_local = Regex::new(
    //     r"^(?:file:///(?P<file_path>[^ \r\n]+)|[./~][^ \r\n]*|[A-Za-z]:[\\/][^ \r\n]*)$"
    // ).unwrap();

    re_scheme
        .captures(url)
        .or(re_scp.captures(url))
        .ok_or(ParseUrlError(url.to_string()))
    //.or(re_local.captures(url))
}

#[derive(Debug, Clone, Hash, PartialEq, Serialize, Deserialize, Default)]
/// Repository Identifier
pub struct RepoId<'config> {
    /// Remote URL of the repository. This is the URL of the remote which is
    /// used to deduce the repository path in the repo tree, in case your
    /// repository has several ones. None if the repository is local.
    pub remote_url: Option<String>,
    #[serde(skip)]
    /// Information about the host associated with the repository.
    pub host: Host<'config>,
    /// Name of the repository.
    pub name: String,
}

impl<'config> RepoId<'config> {
    /// Parse the provided repository remote URL into a host (as Host enum) and
    /// the local path the repository should be located at in the repo tree
    /// based according to the URL.
    pub fn parse_url(
        config: &'config Config,
        remote_url: &str,
    ) -> Result<Self, ParseUrlError> {
        let remote_cap = capture_url(remote_url)?;
        let host_url = &remote_cap["host"];
        let name = remote_cap["path"].to_string();

        let host = config.get_remote_host(host_url).map_or(
            Host::unknown_remote(config, host_url.to_owned()),
            Host::Remote,
        );

        Ok(Self {
            remote_url: Some(remote_url.to_string()),
            host,
            name,
        })
    }

    /// Parse the provided repository remote URL into a host (as Host struct)
    /// and the local path the repository should be located at in the repo
    /// tree based according to the URL.
    /// This version (in regard to parse_url()) defaults to the local host
    /// location configuration if the remote_url argument is None.
    pub fn parse_repo_url<P: AsRef<Path>>(
        config: &'config Config,
        repo_path: &P,
        remote_url: Option<&String>,
    ) -> Result<RepoId<'config>, ParseUrlError> {
        if let Some(remote_url) = remote_url {
            Self::parse_url(config, remote_url)
        } else {
            Ok(Self {
                remote_url: None,
                host: Host::local(config),
                name: compute_local_path(&config.repo_tree_dir, repo_path),
            })
        }
    }

    /// Get the path in the repo tree, where the repository should be located.
    pub fn location(
        &self,
        config: &Config,
    ) -> Result<PathBuf, UnknownRemoteHostError> {
        self.host.get_host_dir(config).map(|p| p.join(&self.name))
    }
}

impl<'config> Display for RepoId<'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.host, self.name)?;
        if let Some(remote_url) = &self.remote_url {
            write!(f, " {remote_url}")?;
        }

        Ok(())
    }
}
