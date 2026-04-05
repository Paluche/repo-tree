//! Functions related to git submodules.
use std::error::Error;
use std::fs::canonicalize;
use std::path::Path;
use std::path::PathBuf;

use git2::Oid;
use regex::Regex;
use url::Url;

/// Resolve a submodule URL that may be relative into an absolute URL,
/// using the repository's remote URL as the base.
///
/// This handles:
/// - absolute URLs (http://, https://, ssh://, git://, file://, etc.) =>
///   returned as-is
/// - scp-style urls (git@host:owner/repo.git). If `submodule_url` is relative
///   (starts with ./ or ../), it is resolved against the base scp path.
/// - relative pathlike urls (../foo/bar) resolved against the base remote path.
///
/// Notes / caveats:
/// - Mirrors typical git behavior but is not an exact implementation of all git
///   heuristics, the remote being deduced in order as:
///     1) "origin"
///     2) the first remote defined
/// - Git has more subtle behavior (e.g. remote used for the branch you fetch
///   from, config fallbacks).
/// - scp-style handling is implemented heuristically by converting the scp url
///   into an ssh:// URL for path joining.
/// - This helper focuses on the common cases; for complete fidelity to `git`
///   behaviour, call git itself or replicate git's source logic.
///
/// Returns: resolved URL string (absolute).
fn resolve_url<P: AsRef<Path>>(
    base_repo_path: P,
    base_remote_url: Option<String>,
    submodule_url: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    // If it already looks like an absolute URL with a scheme, return as-is.
    if submodule_url.contains("://") {
        return Ok(Some(submodule_url.to_string()));
    }

    // If it's an absolute filesystem path, return as-is
    if Path::new(submodule_url).is_absolute() {
        return Ok(Some(submodule_url.to_string()));
    }

    // Pick a base remote to resolve against
    // If base remote is scp-like (user@host:path) convert it to
    // ssh://user@host/ for joining
    let scp_re =
        Regex::new(r"^(?:(?P<user>[^@]+)@)?(?P<host>[^:]+):(?P<path>.+)$")
            .unwrap();

    Ok(if let Some(base_remote_url) = base_remote_url {
        // Try to parse base as a normal URL (http/https/git/file)
        if let Ok(mut base) = Url::parse(&base_remote_url) {
            // We want to join against the parent directory of the repository's
            // path component
            // e.g. base path: /owner/repo.git -> parent: /owner/
            let base_path = base.path();
            let base_parent = Path::new(base_path)
                .parent()
                .map(|p| p.to_string_lossy())
                .unwrap_or_else(|| "".into());

            // Build a base URL whose path is the base_parent, so joining
            // 'submodule_url' works.
            // Ensure trailing slash to treat it as a directory for Url::join
            base.set_path(&format!("{}/", base_parent.trim_end_matches('/')));

            let joined = base.join(submodule_url)?;
            Some(joined.to_string())
        } else if scp_re.is_match(&base_remote_url) {
            // parse base as scp-like
            let caps = scp_re.captures(&base_remote_url).unwrap();
            let user = caps.name("user").map(|m| m.as_str());
            let host = caps.name("host").unwrap().as_str();
            let base_path = caps.name("path").unwrap().as_str();

            // Convert base to ssh:// URL so we can use url::Url for path
            // manipulation.
            let mut base_ssh = if let Some(u) = user {
                format!("ssh://{}@{}/", u, host)
            } else {
                format!("ssh://{}/", host)
            };
            // Ensure base_path is treated as path and not as host; join parent
            // directory of base_path
            // e.g. base_path = "owner/repo.git" -> base_dir = "owner"
            let base_parent = Path::new(base_path)
                .parent()
                .map(|p| p.to_string_lossy())
                .unwrap_or_else(|| "".into());
            if !base_parent.is_empty() {
                // note: url::Url::join works with trailing slashes; ensure one
                // exists
                base_ssh.push_str(&format!("{}/", base_parent));
            }

            // Now join the relative submodule_url to this base_ssh
            let base = Url::parse(&base_ssh)?;
            let joined = base.join(submodule_url)?;
            // Convert ssh://user@host/... back to scp-like (optional). We will
            // return ssh:// form which is valid.
            Some(joined.to_string())
        } else {
            // Fallback: if base_remote_url is just a local path or otherwise
            // unparseable, resolve path-like by treating base_remote_url as a
            // directory.
            resolve_url_as_relpath(base_remote_url, submodule_url)
        }
    } else {
        resolve_url_as_relpath(base_repo_path, submodule_url)
    })
}

/// Resolve the submodule URL which is configured as a relative path / URL.
fn resolve_url_as_relpath<P: AsRef<Path>>(
    repo_path: P,
    submodule_url: &str,
) -> Option<String> {
    canonicalize(
        repo_path
            .as_ref()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| Path::new("").to_path_buf())
            .join(submodule_url),
    )
    .ok()
    .map(|p| p.to_string_lossy().to_string())
}

/// Information on a submodule.
#[allow(unused)]
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
    main_repo_remote_url: Option<String>,
) -> Result<Vec<SubmoduleInfo>, Box<dyn Error>> {
    let main_repo_root = main_repo_root.as_ref().to_path_buf();
    let repo = git2::Repository::discover(&main_repo_root)?;
    let submodules = repo.submodules()?;
    let mut ret = Vec::new();

    for submodule in submodules {
        let sub_path = submodule.path().to_path_buf();
        ret.push(if let Some(conf_url) = submodule.url() {
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
                url: resolve_url(
                    &main_repo_root,
                    main_repo_remote_url.clone(),
                    conf_url,
                )?,
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
