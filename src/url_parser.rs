//! Tools around parsing of repositories URL.
use crate::config::Config;
use regex::Regex;

enum HostWorkDir {
    Missing(String),
    Resolved(String),
}

impl HostWorkDir {
    fn into_option(self) -> Option<String> {
        match self {
            Self::Missing(_) => None,
            Self::Resolved(res) => Some(res),
        }
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

    fn get_host_work_dir(&self, host: &str) -> HostWorkDir {
        self.config.get_host(host).cloned().map_or(
            HostWorkDir::Missing(String::from(host)),
            HostWorkDir::Resolved,
        )
    }

    fn parse_url<'b>(url: &'b str) -> Option<regex::Captures<'b>> {
        // scheme-based URLs, e.g.:
        //   https://github.com/owner/repo.git
        //   ssh://user@host:2222/owner/repo.git
        //   git://host/owner/repo
        //   file:///path/to/repo.git
        // Captures: scheme, user (optional), host, port (optional), path
        let re_scheme = Regex::new(concat!(
            r"^(?P<scheme>(?:git|ssh|https?|git\+ssh|rsync|file))",
            r"://(?:(?P<user>[^@/:]+)@)?(?P<host>[^/:]+)",
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

    pub fn parse(
        &self,
        remote_url: Option<&String>,
    ) -> Option<(Option<String>, String)> {
        let remote_cap = Self::parse_url(remote_url?)?;
        let host_work_dir = self.get_host_work_dir(&remote_cap["host"]);

        if let HostWorkDir::Missing(host) = &host_work_dir
            && !self.missing_hosts.contains(host)
        {
            eprintln!("Missing host configuration for {host}");
        }

        Some((host_work_dir.into_option(), remote_cap["path"].to_string()))
    }
}
