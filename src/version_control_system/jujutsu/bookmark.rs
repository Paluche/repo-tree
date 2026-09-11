//! Get the bookmarks status for a repository.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::process::Command;

use colored::Colorize;

use crate::error::CommandError;

/// Representation of a bookmark.
pub struct Bookmark {
    /// Name of the Bookmark.
    name: String,
    /// Target of the local bookmark.
    local_target: Option<String>,
    /// The bookmark is deleted locally and pending to be deleted remotely with
    /// a push operation or to be forgotten about.
    pub deleted: bool,
    /// If the bookmark has been modified compared to one of its known remote
    /// state.
    modified: bool,
    /// If the bookmark is a remotely, you will find here the commit ID where
    /// it is at and the name of the remote for each remote.
    remotes: Vec<String>,
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
    pub fn get_repr(&self) -> Vec<String> {
        if self.is_remote_only() {
            self.remotes
                .iter()
                .map(|remote| {
                    format!("{}@{}", self.name.as_str(), remote)
                        .purple()
                        .to_string()
                })
                .collect()
        } else if self.is_local_only() {
            Vec::from([self.name.as_str().bright_green().to_string()])
        } else {
            Vec::from([format!(
                "{}{}",
                self.name.as_str(),
                if self.modified { "*" } else { "" }
            )
            .bright_purple()
            .to_string()])
        }
    }
}

/// How we return the bookmarks of a repository.
pub type Bookmarks = HashMap<String, Bookmark>;

/// Get the bookmarks the repository currently have.
pub fn get_bookmarks(repo_path: &Path) -> Result<Bookmarks, Box<dyn Error>> {
    let template = [
        "name",
        "remote",
        r#"if(normal_target, normal_target.change_id(), "")"#,
        "synced",
        "conflict",
    ]
    .join(r#"++ "|" ++ "#)
        + r#"++ "\n""#;

    struct Line {
        name: String,
        remote: Option<String>,
        target: Option<String>,
        synced: bool,
        conflict: bool,
    }

    impl Line {
        fn from_line(line: &str) -> Self {
            fn string_part(part: &str) -> String {
                part.to_string()
            }

            fn option_string_part(part: &str) -> Option<String> {
                match part {
                    "" => None,
                    value => Some(value.to_string()),
                }
            }

            fn bool_part(part: &str) -> bool {
                match part {
                    "true" => true,
                    "false" => false,
                    _ => panic!("Should not happen"),
                }
            }

            let mut parts = line.split("|");

            let name = string_part(parts.next().unwrap());
            let remote = option_string_part(parts.next().unwrap());
            let target = option_string_part(parts.next().unwrap());
            let synced = bool_part(parts.next().unwrap());
            let conflict = bool_part(parts.next().unwrap());

            Self {
                name,
                remote,
                target,
                synced,
                conflict,
            }
        }

        fn into_bookmark(self) -> Bookmark {
            Bookmark {
                name: self.name,
                deleted: if self.remote.is_none() && !self.conflict {
                    self.target.is_none()
                } else {
                    false
                },
                local_target: if self.remote.is_none() {
                    self.target
                } else {
                    None
                },
                modified: !self.synced,
                remotes: if let Some(remote) = self.remote {
                    Vec::from([remote])
                } else {
                    Vec::new()
                },
            }
        }

        fn merge_with_bookmark(&self, bookmark: &mut Bookmark) {
            if let Some(remote) = &self.remote {
                bookmark.remotes.push(remote.clone());
            } else {
                bookmark.local_target = self.target.clone();
            }
            bookmark.modified |= !self.synced;
        }
    }

    let mut command = Command::new("jj");
    let output = command
        .arg("--repository")
        .arg(repo_path)
        .arg("bookmark")
        .arg("list")
        .arg("--all")
        .arg("--template")
        .arg(template)
        .output()?;

    if !output.status.success() {
        return Err(Box::new(CommandError::new(command, output)));
    }

    let lines: Vec<Line> = String::from_utf8(output.stdout)?
        .split("\n")
        .filter(|l| !l.is_empty())
        .map(Line::from_line)
        .collect();

    let mut ret = HashMap::new();

    for line in lines {
        ret.entry(line.name.clone())
            .and_modify(|b| line.merge_with_bookmark(b))
            .or_insert(line.into_bookmark());
    }

    Ok(ret)
}
