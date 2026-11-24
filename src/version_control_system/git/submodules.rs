//! Functions related to git submodules.
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use git2::Oid;

use crate::repo_id::Remote;

/// Resolve a submodule URL that may be relative into an absolute URL, the
/// way git does when initializing the submodule.
///
/// A URL starting with `./` or `../` is relative to the URL of the remote of
/// the main repository (its "origin" remote or, if none, the first one), the
/// main repository root directory being used when it has no remote. As for
/// git, the remote URL is considered a directory: each leading `../` removes
/// a component of it, and the remaining URL is appended. Other URLs are
/// returned unchanged.
///
/// Returns `None` if the relative URL goes above the root of the remote URL.
fn resolve_url<P: AsRef<Path>>(
    main_repo_root: P,
    main_repo_remote: &Option<Remote>,
    submodule_url: &str,
) -> Option<String> {
    if !(submodule_url.starts_with("./") || submodule_url.starts_with("../")) {
        return Some(submodule_url.to_string());
    }

    let base = main_repo_remote.as_ref().map_or_else(
        || main_repo_root.as_ref().to_string_lossy().into_owned(),
        |remote| remote.url.clone(),
    );

    join_relative_url(&base, submodule_url)
}

/// Join the relative URL `url` to `base`, following git rules: `base` is
/// considered as a directory, each leading `../` of `url` removes one
/// component of it and each leading `./` is ignored.
///
/// The scheme and host of a URL (`https://host`), or the host of a scp-like
/// URL (`user@host:`), are never removed. Returns `None` if `url` would go
/// above them.
fn join_relative_url(base: &str, url: &str) -> Option<String> {
    // Length of the part of the base which cannot be removed.
    let protected = if let Some(scheme_end) = base.find("://") {
        let host_start = scheme_end + 3;
        base[host_start..]
            .find('/')
            .map_or(base.len(), |i| host_start + i)
    } else if let Some(colon) = base.find(':')
        && !base[..colon].contains('/')
    {
        colon + 1
    } else {
        usize::from(base.starts_with('/'))
    };

    let mut base = base.trim_end_matches('/').to_string();
    let mut url = url;
    loop {
        if let Some(rest) = url.strip_prefix("./") {
            url = rest;
        } else if let Some(rest) = url.strip_prefix("../") {
            if base.len() <= protected {
                return None;
            }
            let last = base[protected..]
                .rfind('/')
                .map_or(protected, |i| protected + i);
            base.truncate(last);
            url = rest;
        } else {
            break;
        }
    }

    Some(if base.ends_with(':') || base.is_empty() {
        format!("{base}{url}")
    } else {
        format!("{base}/{url}")
    })
}

/// Information on a submodule.
pub struct SubmoduleInfo {
    /// Path to the root of the main repository.
    pub main_repo_root: PathBuf,
    /// Relative path from the main repository to the submodule.
    pub sub_path: PathBuf,
    /// Head OID, commit at which the submodule is configured to be.
    pub head: Option<Oid>,
    /// Head OID, the submodule is currently at.
    pub actual_head: Option<Oid>,
    /// Number of commits ahead to go to the head to the action_head.
    pub ahead: Option<usize>,
    /// Number of commits behind to go to the head to the action_head.
    pub behind: Option<usize>,
    /// URL of the submodule remote as configured in the .gitmodules file.
    pub config_url: Option<String>,
    /// Resolved URL of the submodule.
    pub url: Option<String>,
}

impl SubmoduleInfo {
    /// Obtain the absolute path of the submodule.
    pub fn abs_path(&self) -> PathBuf {
        self.main_repo_root.join(&self.sub_path)
    }
}

/// Get the submodules information.
pub fn get<P: AsRef<Path>>(
    main_repo_root: P,
    main_repo_remote: &Option<Remote>,
) -> Result<Vec<SubmoduleInfo>, Box<dyn Error>> {
    let main_repo_root = main_repo_root.as_ref().to_path_buf();
    let repo = git2::Repository::discover(&main_repo_root)?;
    let submodules = repo.submodules()?;
    let mut ret = Vec::new();

    for submodule in submodules {
        let sub_path = submodule.path().to_path_buf();
        ret.push(if let Some(conf_url) = submodule.url()? {
            let head = submodule.head_id();
            let actual_head = submodule.index_id();
            let (ahead, behind) = if let (Some(head), Some(actual_head)) =
                (head, actual_head)
            {
                if let (Ok(head_obj), Ok(actual_obj)) =
                    (repo.find_commit(head), repo.find_commit(actual_head))
                {
                    let (a, b) = repo
                        .graph_ahead_behind(actual_obj.id(), head_obj.id())?;
                    (Some(a), Some(b))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            SubmoduleInfo {
                main_repo_root: main_repo_root.clone(),
                sub_path,
                head,
                actual_head,
                ahead,
                behind,
                config_url: Some(conf_url.to_string()),
                url: resolve_url(&main_repo_root, main_repo_remote, conf_url),
            }
        } else {
            SubmoduleInfo {
                main_repo_root: main_repo_root.clone(),
                sub_path,
                head: None,
                actual_head: None,
                ahead: None,
                behind: None,
                config_url: None,
                url: None,
            }
        })
    }

    Ok(ret)
}
