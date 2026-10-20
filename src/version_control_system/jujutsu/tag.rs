//! Get the tags status for a repository.

use std::error::Error;
use std::path::Path;

use super::revset::run_revset;
use super::revset::RevSetOrder;
use super::command::JujutsuCommand;
use crate::config::JujutsuTagConfig;

/// Representation of a tag.
#[derive(Debug)]
pub struct Tag {
    /// Name of the tag.
    pub name: String,
    /// The local tag is conflicted.
    conflicted: bool,
    /// Target of the local tag.
    local_target: Option<String>,
    /// The tag is deleted locally and pending to be deleted remotely with
    /// a push operation.
    deleted: bool,
    /// If the tag has been modified compared to one of its known remote
    /// state.
    modified: bool,
    /// If the tag is a remote one, you will find here the commit ID where
    /// it is at and the name of the remote for each remote.
    remotes: Vec<(String, String)>,
}

impl Tag {
    /// Get the tag short representation.
    pub fn get_repr(
        self: &Tag,
        tag_config: &JujutsuTagConfig,
    ) -> Vec<String> {
        let name = if self.conflicted {
            &format!("{}??", self.name)
        } else {
            &self.name
        };

        if self.local_target.is_some() && self.remotes.is_empty() {
            // Tag exists only locally.
            Vec::from([tag_config.local.colorize(name)])
        } else {
            // Tag exists on some remotes or all remotes but not locally.
            self
            .remotes
            .iter()
            .map(|remote| tag_config.remote.colorize(name, &remote.0))
            .collect()
        }
    }
}

/// How we return the tags of a repository.
pub struct Tags {
    tags: Vec<Tag>
}

impl Tags {
    fn push(&mut self, tag: Tag) {
        self.tags.push(tag);
    }

    fn get_mut(&mut self, name: &String) -> Option<&mut Tag> {
        self.tags.iter_mut().find(|b| &b.name == name)
    }

    fn get(&self, tag: TagRaw) -> Option<&Tag> {
        let local = tag.remote.is_none();

        self.tags.iter().find(|b| {
            (b.name == tag.name)
                && ((local && b.local_target == tag.target)
                    || (!local
                        && b.remotes
                            .iter()
                            .any(|(_, t)| Some(t) == tag.target.as_ref())))
        })
    }

    pub fn get_deleted(&self) -> Vec<&Tag> {
        self.tags.iter().filter(|b| b.deleted).collect()
    }
}


struct TagRaw {
    name: String,
    remote: Option<String>,
    target: Option<String>,
    synced: bool,
    conflict: bool,
}

impl TagRaw {
    fn template_fields(prefix: &str, suffix: &str) -> [String; 5] {
        [
            format!("{prefix}name{suffix}"),
            format!("{prefix}remote{suffix}"),
            format!(
                r#"if({prefix}normal_target{suffix}, {prefix}normal_target{suffix}.change_id(), "")"#
            ),
            format!("{prefix}synced{suffix}"),
            format!("{prefix}conflict{suffix}"),
        ]
    }

    fn tag_list_template() -> String {
        Self::template_fields("", "").join(r#"++ "|" ++ "#) + r#"++ "\n""#
    }

    fn log_template() -> String {
        // Using filter in the template. Keep remote branches only when there is a
        // no local Bookmark tracking it.
        format!(
            r#"bookmarks.filter(|b| !b.remote() || !b.tracking_present()).map(|b| {}).join("\n")"#,
            Self::template_fields("b.", "()").join(r#"++ "|" ++ "#)
        )
    }

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
                self.target.clone()
            } else {
                None
            },
            modified: !self.synced,
            remotes: if let Some(remote) = self.remote {
                Vec::from([(remote, self.target.unwrap())])
            } else {
                Vec::new()
            },
        }
    }

    fn merge_with_tag(&self, tag: &mut Tag) {
        if let Some(remote) = &self.remote {
            tag.remotes.push((remote.clone(), self.target.as_ref().unwrap().clone()));
        } else {
            tag.local_target = self.target.clone();
        }
        tag.modified |= !self.synced;
    }
}

/// Get the tags the repository currently have.
pub fn get_all_tags(repo_path: &Path) -> Result<Tags, Box<dyn Error>> {
    let lines: Vec<TagRaw> = JujutsuCommand::repo(repo_path)?
        .arg("tag")
        .arg("list")
        .arg("--all")
        .arg("--template")
        .arg(TagRaw::tag_list_template())
        .output_lines()?
        .iter()
        .map(|l| TagRaw::from_line(l))
        .collect();

    let mut ret = Tags { tags: Vec::new()};

    for line in lines {
        if line.remote.is_some()
            && let Some(tag) = ret.get_mut(&line.name)
        {
            line.merge_with_tag(tag);
        } else {
            ret.push(line.into_tag());
        }
    }

    Ok(ret)
}

pub fn get_tags<'tags>(
    repo_path: &Path,
    tags: &'tags Tags,
    revset: &str,
    order: RevSetOrder,
) -> Result<Vec<&'tags Tag>, Box<dyn Error>> {
    Ok(
        run_revset(repo_path, revset, &TagRaw::log_template(), order)?
            .iter()
            .filter_map(|l| tags.get(TagRaw::from_line(l)))
            .collect(),
    )
}
