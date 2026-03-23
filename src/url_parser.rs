//! Tools around parsing of repositories URL.
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::{
    config::{Config, Host},
    error::ParseUrlError,
};

enum HostWorkDir {
    Missing(String),
    Resolved(Host),
}

impl HostWorkDir {
    fn into_option(self) -> Option<Host> {
        match self {
            Self::Missing(_) => None,
            Self::Resolved(res) => Some(res),
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

fn get_host_work_dir(config: &Config, host_url: &str) -> HostWorkDir {
    config.get_host(host_url).cloned().map_or_else(
        || HostWorkDir::Missing(host_url.to_string()),
        HostWorkDir::Resolved,
    )
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

/// Parse the provided repository remote URL into a host (as Host struct)
/// and the local path the repository should be located at in the repo
/// tree based according to the URL.
pub fn parse_url(
    config: &Config,
    remote_url: &str,
) -> Result<(Option<Host>, String), ParseUrlError> {
    let remote_cap = capture_url(remote_url)?;
    let host_repo_tree_dir = get_host_work_dir(config, &remote_cap["host"]);

    if let HostWorkDir::Missing(host) = &host_repo_tree_dir {
        eprintln!("Missing host configuration for {host}");
    }

    Ok((
        host_repo_tree_dir.into_option(),
        remote_cap["path"].to_string(),
    ))
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
) -> Result<(Option<Host>, String), ParseUrlError> {
    if let Some(remote_url) = remote_url {
        parse_url(config, remote_url)
    } else {
        Ok((
            Some(config.local.clone()),
            compute_local_path(&config.repo_tree_dir, repo_path),
        ))
    }
}
