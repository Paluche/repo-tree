//! Tools around parsing of repositories URL.
use std::{collections::HashMap, path::Path};
use url::Url;

pub struct UrlParser<'a> {
    hosts: HashMap<&'a str, &'a str>,
}

impl<'a> UrlParser<'a> {
    fn get_host_work_dir(&self, host: &str) -> Option<&str> {
        self.hosts.get(host).copied()
    }

    fn _parse(
        &self,
        remote_url: Option<&String>,
    ) -> Option<(Option<String>, String)> {
        let url = remote_url?;
        let (_user, url) = if let Some(parse) = url.split_once("@") {
            (Some(parse.0), parse.1)
        } else {
            (None, url.as_str())
        };

        let url = Url::parse(url).ok()?;

        let host = url.host_str();
        let host_work_dir = host
            .and_then(|x| self.get_host_work_dir(x))
            .or(self.get_host_work_dir(url.scheme()))
            .map(String::from);
        let path = url.path().to_owned();

        if host_work_dir.is_none() {
            eprintln!("Missing host configuration for {url}");
        }

        if path.ends_with(".git") {
            Some((
                host_work_dir,
                path.strip_suffix(".git").unwrap().to_string(),
            ))
        } else {
            Some((host_work_dir, path))
        }
    }

    pub fn parse<P: AsRef<Path>>(
        &self,
        remote_url: Option<&String>,
        repo_path: &P,
    ) -> (Option<String>, String) {
        self._parse(remote_url).unwrap_or((
            Some("local".to_string()),
            repo_path
                .as_ref()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        ))
    }
}

impl<'a> Default for UrlParser<'a> {
    fn default() -> Self {
        let hosts = HashMap::from([
            ("github.com", "github"),
            ("gitlab.com", "gitlab"),
            ("git.kernel.org", "kernel"),
            ("git.buildroot.net", "."),
            ("bitbucket.org", "bitbucket"),
        ]);
        Self { hosts }
    }
}
