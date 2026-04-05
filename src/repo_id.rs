//! Tools around parsing of repositories URL.
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::error::ParseUrlError;
use crate::error::UnknownRemoteHostError;
use crate::host::Host;
use crate::host::Remote;

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

#[derive(Debug, Clone, Hash, PartialEq, Serialize, Deserialize)]
/// Repository Identifier
pub struct RepoId {
    /// Information about the host associated with the repository.
    pub remote: Remote,
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
            remote: Remote::Remote(
                remote_url.to_string(),
                host_url.to_string(),
            ),
            name,
        })
    }

    /// Parse the provided repository remote URL into a host (as Remote struct)
    /// and the local path the repository should be located at in the repo
    /// tree based according to the URL.
    /// This version (in regard to parse_url()) defaults to the local host
    /// location configuration if the remote_url argument is None.
    pub fn from_repo<P: AsRef<Path>>(
        config: &Config,
        repo_path: &P,
        remote_url: Option<&String>,
    ) -> Result<RepoId, ParseUrlError> {
        if let Some(remote_url) = remote_url {
            Self::from_remote_url(remote_url)
        } else {
            Ok(Self {
                remote: Remote::Local,
                name: compute_local_path(&config.repo_tree_dir, repo_path),
            })
        }
    }

    /// Get the path in the repo tree, where the repository should be located.
    pub fn location(
        &self,
        config: &Config,
    ) -> Result<PathBuf, UnknownRemoteHostError> {
        self.remote
            .host(config)
            .dir_path(config)
            .map(|p| p.join(&self.name))
    }

    /// Obtain a struct implementing the display for the RepoId.
    pub fn display<'repo_id, 'config>(
        &'repo_id self,
        config: &'config Config,
    ) -> RepoIdDisplay<'repo_id, 'config> {
        RepoIdDisplay {
            repo_id: self,
            host: self.remote.host(config),
        }
    }
}

/// Struct to display a RepoId.
pub struct RepoIdDisplay<'repo_id, 'config> {
    /// RepoId to display.
    repo_id: &'repo_id RepoId,
    /// Host data of the RepoId.
    host: Host<'config>,
}

impl<'repo_id, 'config> Display for RepoIdDisplay<'repo_id, 'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.host, self.repo_id.name)?;
        if let Some(remote_url) = &self.repo_id.remote.url() {
            write!(f, " {remote_url}")?;
        }

        Ok(())
    }
}
