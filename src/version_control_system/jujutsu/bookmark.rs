//! Get the bookmarks status for a repository.

use std::error::Error;
use std::path::Path;

use super::GIT_REMOTE;
use super::command::JujutsuCommand;
use super::revset::RevSetOrder;
use super::revset::run_revset;
use crate::config::JujutsuBookmarkConfig;

/// Representation of a bookmark.
pub struct Bookmark {
    /// Name of the Bookmark.
    pub name: String,
    /// The local tracked bookmark is conflicted.
    conflicted: bool,
    /// Target of the local bookmark.
    local_target: Option<String>,
    /// The bookmark is deleted locally and pending to be deleted remotely with
    /// a push operation or to be forgotten about.
    deleted: bool,
    /// If the bookmark has been modified compared to one of its known remote
    /// state.
    modified: bool,
    /// If the bookmark is a remote one, you will find here the commit ID where
    /// it is at and the name of the remote for each remote.
    remotes: Vec<(String, Option<String>)>,
}

impl Bookmark {
    /// Find out if the bookmark has no local tracked part and exists only
    /// remotely.
    fn is_remote_only(&self) -> bool {
        self.local_target.is_none() && !self.remotes.is_empty()
    }

    /// Find out if the bookmark exists only locally.
    fn is_local_only(&self) -> bool {
        self.local_target.is_some() && self.remotes.is_empty()
    }

    /// Get the bookmark short representation.
    pub fn get_repr(
        &self,
        bookmark_config: &JujutsuBookmarkConfig,
    ) -> Vec<String> {
        if self.conflicted {
            Vec::from([bookmark_config
                .tracked
                .colorize(format!("{}??", self.name))])
        } else if self.is_remote_only() {
            self.remotes
                .iter()
                .map(|remote| {
                    bookmark_config
                        .untracked
                        .colorize(self.name.as_str(), &remote.0)
                })
                .collect()
        } else if self.is_local_only() {
            Vec::from([bookmark_config.local.colorize(&self.name)])
        } else {
            Vec::from([bookmark_config.tracked.colorize(format!(
                "{}{}",
                self.name.as_str(),
                if self.modified { "*" } else { "" }
            ))])
        }
    }
}

/// How we return the bookmarks of a repository.
pub struct Bookmarks {
    /// All the bookmarks of the repository.
    bookmarks: Vec<Bookmark>,
}

impl Bookmarks {
    /// Add a bookmark to the list.
    fn push(&mut self, bookmark: Bookmark) {
        self.bookmarks.push(bookmark);
    }

    /// Get the first bookmark with the provided name.
    fn get_mut(&mut self, name: &str) -> Option<&mut Bookmark> {
        self.bookmarks.iter_mut().find(|b| b.name == name)
    }

    /// Find the bookmark matching a raw bookmark from a revset output: same
    /// name and, depending on the raw bookmark being local or remote, same
    /// local target or same remote and target.
    fn get(&self, bookmark: &BookmarkRaw) -> Option<&Bookmark> {
        self.bookmarks.iter().find(|b| {
            b.name == bookmark.name
                && match &bookmark.remote {
                    None => b.local_target == bookmark.target,
                    Some(remote) => b
                        .remotes
                        .iter()
                        .any(|(r, t)| r == remote && t == &bookmark.target),
                }
        })
    }

    /// Get the bookmarks deleted locally, pending to be pushed or forgotten.
    pub fn get_deleted(&self) -> Vec<&Bookmark> {
        self.bookmarks.iter().filter(|b| b.deleted).collect()
    }
}

/// A bookmark as output by jj: either the local bookmark or one of its remote
/// bookmarks.
struct BookmarkRaw {
    /// Name of the bookmark.
    name: String,
    /// Name of the remote, `None` for the local bookmark.
    remote: Option<String>,
    /// Change ID of the target, `None` if absent or conflicted.
    target: Option<String>,
    /// The remote bookmark is in sync with the local one.
    synced: bool,
    /// The remote bookmark is tracked by the local one.
    tracked: bool,
    /// The bookmark target is conflicted.
    conflict: bool,
}

impl BookmarkRaw {
    /// Template fields to output a bookmark, each wrapped with `prefix` and
    /// `suffix` to access them as keywords or methods.
    fn template_fields(prefix: &str, suffix: &str) -> [String; 6] {
        [
            format!("{prefix}name{suffix}"),
            format!("{prefix}remote{suffix}"),
            format!(
                r#"if({prefix}normal_target{suffix}, {prefix}normal_target{suffix}.change_id(), "")"#
            ),
            format!("{prefix}synced{suffix}"),
            format!("{prefix}tracked{suffix}"),
            format!("{prefix}conflict{suffix}"),
        ]
    }

    /// Template for `jj bookmark list`, one bookmark per line.
    fn bookmark_list_template() -> String {
        Self::template_fields("", "").join(r#"++ "|" ++ "#) + r#"++ "\n""#
    }

    /// Template for `jj log`, one bookmark per line for each commit.
    fn log_template() -> String {
        // Using filter in the template. Keep remote branches only when there is
        // no local Bookmark tracking it.
        format!(
            r#"bookmarks.filter(|b| !b.remote() || !b.tracking_present()).map(|b| {}).join("\n")"#,
            Self::template_fields("b.", "()").join(r#"++ "|" ++ "#)
        )
    }

    /// Parse a line output with one of the templates.
    fn from_line(line: &str) -> Result<Self, Box<dyn Error>> {
        /// Parse a boolean field.
        fn bool_part(field: &str, part: &str) -> Result<bool, String> {
            match part {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(format!("invalid {field} value {part:?}")),
            }
        }

        /// Turn an empty field into `None`.
        fn option_string_part(part: &str) -> Option<String> {
            (!part.is_empty()).then(|| part.to_string())
        }

        let parts: Vec<&str> = line.split('|').collect();
        let [name, remote, target, synced, tracked, conflict] = parts[..]
        else {
            return Err(format!(
                "Expected 6 fields in bookmark line {line:?}, got {}",
                parts.len()
            )
            .into());
        };

        let parse_bool = |field, part| {
            bool_part(field, part)
                .map_err(|e| format!("Bad bookmark line {line:?}: {e}"))
        };

        Ok(Self {
            name: name.to_string(),
            remote: option_string_part(remote),
            target: option_string_part(target),
            synced: parse_bool("synced", synced)?,
            tracked: parse_bool("tracked", tracked)?,
            conflict: parse_bool("conflict", conflict)?,
        })
    }

    /// Create a new bookmark from this raw bookmark.
    fn into_bookmark(self) -> Bookmark {
        Bookmark {
            name: self.name,
            deleted: if self.remote.is_none() && !self.conflict {
                self.target.is_none()
            } else {
                false
            },
            conflicted: self.conflict,
            local_target: if self.remote.is_none() {
                self.target.clone()
            } else {
                None
            },
            modified: !self.synced,
            remotes: if let Some(remote) = self.remote {
                Vec::from([(remote, self.target)])
            } else {
                Vec::new()
            },
        }
    }

    /// Merge this raw bookmark into the bookmark with the same name.
    fn merge_with_bookmark(self, bookmark: &mut Bookmark) {
        if let Some(remote) = self.remote {
            bookmark.remotes.push((remote, self.target));
        } else {
            bookmark.local_target = self.target;
        }
        bookmark.modified |= !self.synced;
    }
}

/// Get the bookmarks the repository currently have.
pub fn get_all_bookmarks(
    repo_path: &Path,
) -> Result<Bookmarks, Box<dyn Error>> {
    let lines = JujutsuCommand::repo(repo_path)?
        .arg("bookmark")
        .arg("list")
        .arg("--all")
        .arg("--template")
        .arg(BookmarkRaw::bookmark_list_template())
        .output_lines()?
        .iter()
        .map(|l| BookmarkRaw::from_line(l))
        .collect::<Result<Vec<BookmarkRaw>, _>>()?;

    let mut ret = Bookmarks {
        bookmarks: Vec::new(),
    };

    for line in lines {
        if line.remote.as_deref() == Some(GIT_REMOTE) {
            continue;
        }

        if line.remote.is_some()
            && line.tracked
            && let Some(bookmark) = ret.get_mut(&line.name)
        {
            line.merge_with_bookmark(bookmark);
        } else {
            ret.push(line.into_bookmark());
        }
    }

    Ok(ret)
}

/// Get the bookmarks set on the commits matching the revset.
pub fn get_bookmarks<'bookmarks>(
    repo_path: &Path,
    bookmarks: &'bookmarks Bookmarks,
    revset: &str,
    order: RevSetOrder,
) -> Result<Vec<&'bookmarks Bookmark>, Box<dyn Error>> {
    run_revset(repo_path, revset, &BookmarkRaw::log_template(), order)?
        .iter()
        .map(|l| {
            let raw = BookmarkRaw::from_line(l)?;
            bookmarks.get(&raw).ok_or_else(|| {
                format!(
                    "Bookmark {}{} is not in the list of bookmarks",
                    raw.name,
                    raw.remote.map(|r| format!("@{r}")).unwrap_or_default()
                )
                .into()
            })
        })
        .collect()
}
