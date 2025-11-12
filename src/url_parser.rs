//! Tools around parsing of repositories URL.
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::config::{Config, Host};

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

/// Either the repository is within the ${WORKTREE_DIR}/local directory
/// allowing the user to organize as see fits this directory.
/// Or take the directory name.
fn compute_local_path<P: AsRef<Path>>(
    worktree_dir: &Path,
    repo_path: &P,
) -> String {
    let local_dir = worktree_dir.join("local");
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

pub struct UrlParser<'a> {
    config: &'a Config,
    missing_hosts: Vec<String>,
}

impl<'a> UrlParser<'a> {
    pub fn new(config: &'a Config) -> UrlParser<'a> {
        Self {
            config,
            missing_hosts: Vec::new(),
        }
    }

    fn get_host_work_dir(&self, host_url: &str) -> HostWorkDir {
        self.config.get_host(host_url).cloned().map_or_else(
            || HostWorkDir::Missing(host_url.to_string()),
            HostWorkDir::Resolved,
        )
    }

    fn capture_url<'b>(url: &'b str) -> Option<regex::Captures<'b>> {
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

        re_scheme.captures(url).or(re_scp.captures(url))
        //.or(re_local.captures(url))
    }

    pub fn parse_url(
        &self,
        remote_url: &str,
    ) -> Option<(Option<Host>, String)> {
        let remote_cap = Self::capture_url(remote_url)?;
        let host_worktree_dir = self.get_host_work_dir(&remote_cap["host"]);

        if let HostWorkDir::Missing(host) = &host_worktree_dir
            && !self.missing_hosts.contains(host)
        {
            eprintln!("Missing host configuration for {host}");
        }

        Some((
            host_worktree_dir.into_option(),
            remote_cap["path"].to_string(),
        ))
    }

    pub fn parse_repo_url<P: AsRef<Path>>(
        &self,
        worktree_dir: &Path,
        repo_path: &P,
        remote_url: Option<&String>,
    ) -> (Option<Host>, String) {
        remote_url
            .and_then(|u| self.parse_url(u))
            .unwrap_or_else(|| {
                (
                    Some(self.config.local.clone()),
                    compute_local_path(worktree_dir, repo_path),
                )
            })
    }
}
