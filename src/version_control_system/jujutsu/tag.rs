//! Get the tags status for a repository.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use super::super::VcsRemote;
use super::command::JujutsuCommand;
use crate::config::JujutsuTagConfig;

/// Representation of a tag.
#[expect(dead_code)]
pub struct Tag {
    /// Name of the tag.
    name: String,
    /// The local tag is conflicted.
    conflicted: bool,
    /// Target of the local tag.
    local_target: Option<String>,
    /// The tag is deleted locally and pending to be deleted remotely with
    /// a push operation.
    pub deleted: bool,
    /// If the tag has been modified compared to one of its known remote
    /// state.
    modified: bool,
    /// If the tag is a remote one, you will find here the commit ID where
    /// it is at and the name of the remote for each remote.
    remotes: Vec<String>,
}

enum RemoteTagStatus {
    /// Tag exists only locally.
    Local,
    /// Tag exists on all configured remotes.
    AllRemotes,
    /// Tag does not exists on all configured remotes.
    Remotes,
}

impl RemoteTagStatus {
    fn colorize(
        self,
        config: &JujutsuTagConfig,
        name: &String,
        remote: Option<&String>,
    ) -> String {
        match self {
            Self::Local => config.local.colorize(name),
            Self::AllRemotes => config.remote.name.colorize(name),
            Self::Remotes => config.remote.colorize(name, remote.unwrap()),
        }
    }
}

impl Tag {
    /// Get tag remote status.
    fn remote_status(&self, remotes: Vec<VcsRemote>) -> RemoteTagStatus {
        if self.local_target.is_some() && self.remotes.is_empty() {
            RemoteTagStatus::Local
        } else if remotes.iter().all(|r| self.remotes.contains(&r.name)) {
            RemoteTagStatus::AllRemotes
        } else {
            RemoteTagStatus::Remotes
        }
    }

    /// Get the tag short representation.
    pub fn get_repr(
        self: &Tag,
        tag_config: &JujutsuTagConfig,
        remotes: Vec<VcsRemote>,
    ) -> Vec<String> {
        let remote_status = self.remote_status(remotes);

        if self.conflicted {
            Vec::from([remote_status.colorize(tag_config, &format!("{}??", self.name), None)])
        } else {
            match remote_status {
                RemoteTagStatus::Local | RemoteTagStatus::AllRemotes =>
                    Vec::from([remote_status.colorize(tag_config, &self.name, None)]),
                RemoteTagStatus::Remotes => {
                    self.remotes
                        .iter()
                        .map(|remote| tag_config.remote.colorize(&self.name, remote))
                        .collect()
                }
            }
        }
    }
}

/// How we return the tags of a repository.
pub type Tags = HashMap<String, Tag>;

/// Get the tags the repository currently have.
#[expect(dead_code)]
pub fn get_tags(repo_path: &Path) -> Result<Tags, Box<dyn Error>> {
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

        fn into_tag(self) -> Tag {
            Tag {
                name: self.name,
                deleted: if self.remote.is_none() && !self.conflict {
                    self.target.is_none()
                } else {
                    false
                },
                conflicted: self.conflict,
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

        fn merge_with_tag(&self, tag: &mut Tag) {
            if let Some(remote) = &self.remote {
                tag.remotes.push(remote.clone());
            } else {
                tag.local_target = self.target.clone();
            }
            tag.modified |= !self.synced;
        }
    }

    let lines: Vec<Line> = JujutsuCommand::repo(repo_path)?
        .arg("tag")
        .arg("list")
        .arg("--all")
        .arg("--template")
        .arg(template)
        .output_lines()?
        .iter()
        .map(|l| Line::from_line(l))
        .collect();

    let mut ret = HashMap::new();

    for line in lines {
        ret.entry(line.name.clone())
            .and_modify(|b| line.merge_with_tag(b))
            .or_insert(line.into_tag());
    }

    Ok(ret)
}
