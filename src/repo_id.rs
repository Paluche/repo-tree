//! Tools around parsing of repositories URL.
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::{
    config::{Config, Host},
    error::ParseUrlError,
};

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

#[derive(Debug, Clone, Hash, PartialEq)]
/// Repository Identifier
pub struct RepoId {
    /// Remote URL of the repository. This is the URL of the remote which is
    /// used to deduce the repository path in the repo tree, in case your
    /// repository has several ones. None if the repository is local.
    pub remote_url: Option<String>,
    /// Information about the host associated with the repository, None if the
    /// host has not been resolved, due to missing configuration for that host.
    pub host: Option<Host>,
    /// Name of the repository.
    pub name: String,
}

impl RepoId {
    /// Parse the provided repository remote URL into a host (as Host struct)
    /// and the local path the repository should be located at in the repo
    /// tree based according to the URL.
    pub fn parse_url(
        config: &Config,
        remote_url: &str,
    ) -> Result<Self, ParseUrlError> {
        let remote_cap = capture_url(remote_url)?;
        let host_url = &remote_cap["host"];
        let name = remote_cap["path"].to_string();

        let host = config.get_host(host_url).cloned();

        if host.is_none() {
            eprintln!("Missing host configuration for {host_url}");
        }
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
        config: &Config,
        repo_path: &P,
        remote_url: Option<&String>,
    ) -> Result<RepoId, ParseUrlError> {
        if let Some(remote_url) = remote_url {
            Self::parse_url(config, remote_url)
        } else {
            Ok(Self {
                remote_url: None,
                host: Some(config.local.as_host()),
                name: compute_local_path(&config.repo_tree_dir, repo_path),
            })
        }
    }

    /// Get the path in the repo tree, where the repository should be located.
    pub fn location(&self, config: &Config) -> Option<PathBuf> {
        self.host
            .clone()
            .map(|h| config.repo_tree_dir.join(h.dir_name()).join(&self.name))
    }
}

impl Display for RepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(host) = &self.host {
            let dir_name = host.dir_name();
            if dir_name != "." {
                write!(f, "{dir_name} ")?;
            }
        }
        write!(f, "{}", self.name)?;
        if let Some(remote_url) = &self.remote_url {
            write!(f, " {remote_url}")?;
        }

        Ok(())
    }
}
