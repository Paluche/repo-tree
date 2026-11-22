//! Get the tags status for a repository.

use std::error::Error;
use std::path::Path;

use super::GIT_REMOTE;
use super::command::JujutsuCommand;
use super::revset::RevSetOrder;
use super::revset::run_revset;
use crate::config::JujutsuTagConfig;

/// Representation of a tag.
#[derive(Debug, PartialEq)]
pub struct Tag {
    /// Name of the tag.
    pub name: String,
    /// The local tag is conflicted.
    conflicted: bool,
    /// Target of the local tag.
    local_target: Option<String>,
    /// The tag is deleted locally and pending to be deleted remotely with a
    /// push operation or to be forgotten about.
    deleted: bool,
    /// If the tag has been modified compared to one of its tracked remote
    /// state.
    modified: bool,
    /// Name of the remote and target of the tag on that remote, for each
    /// remote the tag exists on.
    remotes: Vec<(String, Option<String>)>,
}

impl Tag {
    /// Find out if the tag has no local tracked part and exists only remotely.
    fn is_remote_only(&self) -> bool {
        self.local_target.is_none() && !self.remotes.is_empty()
    }

    /// Find out if the tag exists only locally.
    fn is_local_only(&self) -> bool {
        self.local_target.is_some() && self.remotes.is_empty()
    }

    /// Get the tag short representation.
    pub fn get_repr(&self, tag_config: &JujutsuTagConfig) -> Vec<String> {
        if self.conflicted {
            Vec::from([tag_config
                .tracked
                .colorize(&format!("{}??", self.name))])
        } else if self.is_remote_only() {
            self.remotes
                .iter()
                .map(|remote| {
                    tag_config.untracked.colorize(self.name.as_str(), &remote.0)
                })
                .collect()
        } else if self.is_local_only() {
            Vec::from([tag_config.local.colorize(&self.name)])
        } else {
            Vec::from([tag_config.tracked.colorize(&format!(
                "{}{}",
                self.name.as_str(),
                if self.modified { "*" } else { "" }
            ))])
        }
    }
}

/// How we return the tags of a repository.
pub struct Tags {
    /// List of the tags of the repository.
    tags: Vec<Tag>,
}

impl Tags {
    /// Add a tag to the list.
    fn push(&mut self, tag: Tag) {
        self.tags.push(tag);
    }

    /// Get the tag with the specified name, to complete its information.
    fn get_mut(&mut self, name: &str) -> Option<&mut Tag> {
        self.tags.iter_mut().find(|t| t.name == name)
    }

    /// Get the tag matching the raw tag information returned by a revset.
    fn get(&self, tag: &TagRaw) -> Option<&Tag> {
        self.tags.iter().find(|t| {
            t.name == tag.name
                && match &tag.remote {
                    None => t.local_target == tag.target,
                    Some(remote) => t.remotes.iter().any(|(r, target)| {
                        r == remote && *target == tag.target
                    }),
                }
        })
    }

    /// Get the tags deleted locally and pending some action.
    pub fn get_deleted(&self) -> Vec<&Tag> {
        self.tags.iter().filter(|t| t.deleted).collect()
    }
}

/// Raw information about a tag as output by a jj command.
struct TagRaw {
    /// Name of the tag.
    name: String,
    /// Remote of the tag, `None` for the local tag.
    remote: Option<String>,
    /// Change ID the tag points to, `None` if it is absent or conflicted.
    target: Option<String>,
    /// The tag is in sync with its tracked remotes.
    synced: bool,
    /// The remote tag is tracked by the local one.
    tracked: bool,
    /// The tag is conflicted.
    conflict: bool,
}

impl TagRaw {
    /// Template fields to output, in the order [`TagRaw::from_line`] parses
    /// them.
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

    /// Template for `jj tag list`.
    fn tag_list_template() -> String {
        Self::template_fields("", "").join(r#"++ "|" ++ "#) + r#"++ "\n""#
    }

    /// Template for `jj log` listing the tags of each commit.
    fn log_template() -> String {
        // Keep remote tags only when there is no local tag tracking them.
        format!(
            r#"tags.filter(|t| !t.remote() || !t.tracking_present()).map(|t| {}).join("\n")"#,
            Self::template_fields("t.", "()").join(r#"++ "|" ++ "#)
        )
    }

    /// Parse a line output by one of the templates.
    fn from_line(line: &str) -> Result<Self, Box<dyn Error>> {
        /// Parse a boolean template field.
        fn bool_part(part: &str) -> Result<bool, String> {
            match part {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(format!(
                    "Unexpected boolean value {part:?} in tag line"
                )),
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
                "Expected 6 fields in tag line {line:?}, got {}",
                parts.len()
            )
            .into());
        };

        Ok(Self {
            name: name.to_string(),
            remote: option_string_part(remote),
            target: option_string_part(target),
            synced: bool_part(synced)?,
            tracked: bool_part(tracked)?,
            conflict: bool_part(conflict)?,
        })
    }

    /// Convert into a new tag.
    fn into_tag(self) -> Tag {
        let is_local = self.remote.is_none();
        Tag {
            name: self.name,
            deleted: is_local && !self.conflict && self.target.is_none(),
            conflicted: self.conflict,
            local_target: if is_local { self.target.clone() } else { None },
            modified: !self.synced,
            remotes: match self.remote {
                Some(remote) => Vec::from([(remote, self.target)]),
                None => Vec::new(),
            },
        }
    }

    /// Complete an existing tag with this remote tag information.
    fn merge_with_tag(self, tag: &mut Tag) {
        if let Some(remote) = self.remote {
            tag.remotes.push((remote, self.target));
        } else {
            tag.local_target = self.target;
        }
        tag.modified |= !self.synced;
    }
}

impl Tags {
    /// Build the tags of a repository from the raw tags listed by
    /// `jj tag list --all-remotes`, merging each tracked remote tag into its
    /// local tag.
    fn from_raw(raw_tags: Vec<TagRaw>) -> Self {
        let mut ret = Self { tags: Vec::new() };

        for raw in raw_tags {
            if raw.remote.as_deref() == Some(GIT_REMOTE) {
                continue;
            }

            if raw.remote.is_some()
                && raw.tracked
                && let Some(tag) = ret.get_mut(&raw.name)
            {
                raw.merge_with_tag(tag);
            } else {
                ret.push(raw.into_tag());
            }
        }

        ret
    }
}

/// Get all the tags of the repository.
pub fn get_all_tags(repo_path: &Path) -> Result<Tags, Box<dyn Error>> {
    Ok(Tags::from_raw(
        JujutsuCommand::repo(repo_path)?
            .arg("tag")
            .arg("list")
            .arg("--all-remotes")
            .arg("--template")
            .arg(TagRaw::tag_list_template())
            .output_lines()?
            .iter()
            .map(|l| TagRaw::from_line(l))
            .collect::<Result<Vec<TagRaw>, _>>()?,
    ))
}

/// Get the tags set on the commits matching the revset.
pub fn get_tags<'tags>(
    repo_path: &Path,
    tags: &'tags Tags,
    revset: &str,
    order: RevSetOrder,
) -> Result<Vec<&'tags Tag>, Box<dyn Error>> {
    run_revset(repo_path, revset, &TagRaw::log_template(), order)?
        .iter()
        .map(|l| TagRaw::from_line(l).map(|tag| tags.get(&tag)))
        .filter_map(Result::transpose)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::colors;

    /// Parse the lines of a `jj tag list` output.
    fn tags(lines: &[&str]) -> Tags {
        Tags::from_raw(
            lines
                .iter()
                .map(|line| TagRaw::from_line(line).expect("valid line"))
                .collect(),
        )
    }

    /// Representation of every tag.
    fn reprs(lines: &[&str]) -> Vec<String> {
        let config = JujutsuTagConfig::default();
        tags(lines)
            .tags
            .iter()
            .flat_map(|b| b.get_repr(&config))
            .collect()
    }

    #[test]
    fn parse_line() {
        let raw = TagRaw::from_line("main|origin|abc|false|true|false")
            .expect("valid line");

        assert_eq!(raw.name, "main");
        assert_eq!(raw.remote.as_deref(), Some("origin"));
        assert_eq!(raw.target.as_deref(), Some("abc"));
        assert!(!raw.synced);
        assert!(raw.tracked);
        assert!(!raw.conflict);

        let raw =
            TagRaw::from_line("main|||true|false|true").expect("valid line");
        assert_eq!(raw.remote, None);
        assert_eq!(raw.target, None);
        assert!(raw.conflict);
    }

    #[test]
    fn parse_malformed_line_fails() {
        for line in [
            "",
            "main|origin|abc|true|true",
            "main|origin|abc|true|true|false|extra",
            "main|origin|abc|yes|true|false",
            "main|origin|abc|true|1|false",
            "main|origin|abc|true|true|",
        ] {
            assert!(TagRaw::from_line(line).is_err(), "{line:?}");
        }
    }

    #[test]
    fn local_only() {
        let _colors = colors();
        let config = JujutsuTagConfig::default();

        assert_eq!(
            reprs(&["main||abc|true|false|false"]),
            [config.local.colorize("main")]
        );
    }

    #[test]
    fn colocated_git_remote_is_ignored() {
        let _colors = colors();
        let config = JujutsuTagConfig::default();

        assert_eq!(
            reprs(&[
                "main||abc|true|false|false",
                "main|git|abc|true|true|false"
            ]),
            [config.local.colorize("main")]
        );
    }

    #[test]
    fn tracked_remote_is_merged_into_local() {
        let _colors = colors();
        let config = JujutsuTagConfig::default();

        assert_eq!(
            reprs(&[
                "main||abc|true|false|false",
                "main|origin|abc|true|true|false",
            ]),
            [config.tracked.colorize("main")]
        );
        assert_eq!(
            reprs(&[
                "main||abc|true|false|false",
                "main|origin|def|false|true|false",
            ]),
            [config.tracked.colorize("main*")]
        );
    }

    #[test]
    fn untracked_remote_is_displayed_apart() {
        let _colors = colors();
        let config = JujutsuTagConfig::default();

        assert_eq!(
            reprs(&[
                "main||abc|true|false|false",
                "main|upstream|def|true|false|false",
            ]),
            [
                config.local.colorize("main"),
                config.untracked.colorize("main", "upstream"),
            ]
        );
        assert_eq!(
            reprs(&["feature|origin|def|true|false|false"]),
            [config.untracked.colorize("feature", "origin")]
        );
    }

    #[test]
    fn conflicted() {
        let _colors = colors();
        let config = JujutsuTagConfig::default();

        assert_eq!(
            reprs(&["main|||false|false|true"]),
            [config.tracked.colorize("main??")]
        );
        assert_eq!(
            tags(&["main|||false|false|true"]).get_deleted(),
            Vec::<&Tag>::new()
        );
    }

    #[test]
    fn deleted() {
        let all = tags(&[
            "gone|||false|false|false",
            "gone|origin|abc|false|true|false",
            "main||abc|true|false|false",
        ]);

        let deleted: Vec<&str> =
            all.get_deleted().iter().map(|b| b.name.as_str()).collect();
        assert_eq!(deleted, ["gone"]);
    }

    #[test]
    fn get_matching_revset_output() {
        let all = tags(&[
            "main||abc|true|false|false",
            "main|origin|abc|true|true|false",
            "feature|upstream|def|true|false|false",
        ]);
        let get = |line| {
            all.get(&TagRaw::from_line(line).expect("valid line"))
                .map(|b| b.name.clone())
        };

        assert_eq!(get("main||abc|true|false|false").as_deref(), Some("main"));
        assert_eq!(
            get("main|origin|abc|true|true|false").as_deref(),
            Some("main")
        );
        assert_eq!(
            get("feature|upstream|def|true|false|false").as_deref(),
            Some("feature")
        );
        assert_eq!(get("main||other|true|false|false"), None);
        assert_eq!(get("feature|origin|def|true|false|false"), None);
        assert_eq!(get("unknown||abc|true|false|false"), None);
    }
}
