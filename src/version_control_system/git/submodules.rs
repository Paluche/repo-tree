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
    /// Commit at which the main repository HEAD records the submodule.
    pub head: Option<Oid>,
    /// Commit currently checked out in the submodule, `None` when the
    /// submodule is not initialized.
    pub actual_head: Option<Oid>,
    /// Number of commits of `actual_head` that are not in `head`.
    pub ahead: Option<usize>,
    /// Number of commits of `head` that are not in `actual_head`.
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

/// Count the commits of `actual_head` not in `head` and the commits of
/// `head` not in `actual_head`, looked up in the submodule repository, which
/// holds the commits of the submodule (the main repository does not).
///
/// Returns `None` when the submodule repository cannot be opened or does not
/// contain one of the commits, for example before it has been fetched.
fn ahead_behind(
    submodule: &git2::Submodule<'_>,
    actual_head: Oid,
    head: Oid,
) -> Option<(usize, usize)> {
    if actual_head == head {
        return Some((0, 0));
    }
    let repo = submodule.open().ok()?;
    repo.graph_ahead_behind(actual_head, head).ok()
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
            let actual_head = submodule.workdir_id();
            let (ahead, behind) = head
                .zip(actual_head)
                .and_then(|(head, actual_head)| {
                    ahead_behind(&submodule, actual_head, head)
                })
                .unzip();

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Resolve `url` against the main repository remote `remote` (if any),
    /// the main repository being in `/work/main`.
    fn resolve(remote: Option<&str>, url: &str) -> Option<String> {
        resolve_url(
            "/work/main",
            &remote.map(|url| Remote {
                url: url.to_string(),
                host_url: String::new(),
            }),
            url,
        )
    }

    #[test]
    fn absolute_urls_are_unchanged() {
        for url in [
            "https://example.test/owner/sub.git",
            "git@example.test:owner/sub.git",
            "/srv/git/sub.git",
            "file:///srv/git/sub.git",
        ] {
            assert_eq!(
                resolve(Some("https://example.test/o/r.git"), url).as_deref(),
                Some(url)
            );
        }
    }

    #[test]
    fn relative_urls_follow_git_rules() {
        // Expected values obtained with `git submodule init`.
        for (remote, url, expected) in [
            (
                "https://example.test/owner/repo.git",
                "../other.git",
                "https://example.test/owner/other.git",
            ),
            (
                "https://example.test/owner/repo.git",
                "./sub",
                "https://example.test/owner/repo.git/sub",
            ),
            (
                "https://example.test/owner/repo.git",
                "../../top.git",
                "https://example.test/top.git",
            ),
            (
                "https://example.test/owner/repo.git/",
                "../other.git",
                "https://example.test/owner/other.git",
            ),
            (
                "git@example.test:owner/repo.git",
                "../other.git",
                "git@example.test:owner/other.git",
            ),
            (
                "git@example.test:owner/repo.git",
                "./sub",
                "git@example.test:owner/repo.git/sub",
            ),
            (
                "git@example.test:owner/repo.git",
                "../../top.git",
                "git@example.test:top.git",
            ),
            (
                "/srv/git/owner/repo.git",
                "../other.git",
                "/srv/git/owner/other.git",
            ),
            (
                "/srv/git/owner/repo.git",
                "../../top.git",
                "/srv/git/top.git",
            ),
            (
                "ssh://git@example.test:22/owner/repo",
                "./.././other",
                "ssh://git@example.test:22/owner/other",
            ),
        ] {
            assert_eq!(
                resolve(Some(remote), url).as_deref(),
                Some(expected),
                "{remote} + {url}"
            );
        }
    }

    #[test]
    fn relative_url_without_remote_is_relative_to_the_repository() {
        assert_eq!(
            resolve(None, "../lib.git").as_deref(),
            Some("/work/lib.git")
        );
        assert_eq!(resolve(None, "./sub").as_deref(), Some("/work/main/sub"));
    }

    #[test]
    fn relative_url_above_the_host_fails() {
        assert_eq!(
            resolve(Some("https://example.test/r.git"), "../../x"),
            None
        );
        assert_eq!(resolve(Some("git@example.test:r.git"), "../../x"), None);
        assert_eq!(resolve(Some("/r.git"), "../../x"), None);
    }
}
