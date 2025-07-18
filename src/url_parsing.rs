//! Tools around parsing of repositories URL.
use git2::{Remote, Repository};
use std::collections::HashMap;
use url::Url;

fn get_host_work_dir(host: &str) -> Option<&str> {
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
        println!("Unknown host {host}");
        None
    })
}

fn load_default_remote(repo: &Repository) -> Option<Remote> {
    let remotes = repo.remotes().unwrap();

    if remotes.is_empty() {
        None
    } else {
        Some(match repo.find_remote("origin") {
            Ok(remote) => remote,
            Err(_) => repo.find_remote(remotes.get(0)?).unwrap(),
        })
    }
}

pub fn parse_repo_url(repo: &Repository) -> Option<(String, String)> {
    let default_remote = load_default_remote(repo)?;
    let url = String::from(default_remote.url().unwrap());
    let (_user, url) = if let Some(parse) = url.split_once("@") {
        (Some(parse.0), parse.1)
    } else {
        (None, url.as_str())
    };

    let url = Url::parse(url).ok()?;
    let host = url.host_str();
    let host_work_dir = host
        .and_then(get_host_work_dir)
        .or(get_host_work_dir(url.scheme()))?
        .to_owned();
    let mut path = url.path().to_owned();

    if path.ends_with(".git") {
        path = path.strip_suffix(".git").unwrap().to_string();
    }

    Some((host_work_dir, path))
}
