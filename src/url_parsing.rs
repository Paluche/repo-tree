//! Tools around parsing of repositories URL.
use std::{collections::HashMap, path::Path};
use url::Url;

fn get_host_work_dir(host: &str) -> Option<&str> {
    // TODO Add support for custom origins in a JSON or Yaml file.
    HashMap::from([
        ("github.com", "github"),
        ("gitlab.com", "gitlab"),
        ("git.kernel.org", "kernel"),
        ("git.buildroot.net", "."),
        ("bitbucket.org", "bitbucket"),
    ])
    .get(host)
    .copied()
    .or_else(|| {
        eprintln!("Missing configuration for host {host}");
        None
    })
}

fn _parse_repo_url(
    remote_url: Option<&String>,
) -> Option<(Option<String>, String)> {
    if remote_url.is_none() {
        return None;
    }

    let url = remote_url.unwrap();
    let (_user, url) = if let Some(parse) = url.split_once("@") {
        (Some(parse.0), parse.1)
    } else {
        (None, url.as_str())
    };

    let url = Url::parse(url).ok()?;

    let host = url.host_str();
    let host_work_dir = host
        .and_then(get_host_work_dir)
        .or(get_host_work_dir(url.scheme()))
        .map(String::from);
    let path = url.path().to_owned();

    if path.ends_with(".git") {
        Some((
            host_work_dir,
            path.strip_suffix(".git").unwrap().to_string(),
        ))
    } else {
        Some((host_work_dir, path))
    }
}

pub fn parse_repo_url<P: AsRef<Path>>(
    remote_url: Option<&String>,
    repo_path: &P,
) -> (Option<String>, String) {
    _parse_repo_url(remote_url).unwrap_or((
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
