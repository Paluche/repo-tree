//! Configuration for the Jujutsu prompt.

use serde::Deserialize;
use serde::Serialize;

use crate::colors::Color;
use crate::colors::ColoredList;
use crate::colors::ColoredText;

/// Default colored text to represent the Jujutsu version control system.
pub fn default_vcs_prompt() -> ColoredText {
    ColoredText::new("", Color::blue())
}

/// Configuration on how to format a reference with a remote.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RemoteReferenceConfig {
    /// Which color to use for the name of the reference.
    pub name: Color,
    /// Which color to use for the name of the remote.
    pub remote: Color,
}

impl RemoteReferenceConfig {
    pub fn colorize(&self, name: &str, remote: &str) -> String {
        format!(
            "{}{}{}",
            self.name.colorize(name),
            self.remote.colorize("@"),
            self.remote.colorize(remote)
        )
    }
}

/// Configuration for the Jujutsu bookmarks prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct JujutsuBookmarkConfig {
    /// How to color local bookmarks
    #[serde(default = "JujutsuBookmarkConfig::default_local")]
    pub local: Color,
    /// How to color non-tracked remote bookmarks
    #[serde(default = "JujutsuBookmarkConfig::default_untracked")]
    pub untracked: RemoteReferenceConfig,
    /// How to color tracked bookmarks
    #[serde(default = "JujutsuBookmarkConfig::default_tracked")]
    pub tracked: Color,
    /// How to display list of bookmarks set on the parent commit of the
    /// current one we are editing.
    #[serde(default = "JujutsuBookmarkConfig::default_parent")]
    pub parent: ColoredList,
    /// How to display list of bookmarks set on the current commit we are
    /// editing.
    #[serde(default = "JujutsuBookmarkConfig::default_current")]
    pub current: ColoredList,
    /// How to display list of bookmarks set on any of the descendants of the
    /// current commit we are editing.
    #[serde(default = "JujutsuBookmarkConfig::default_descendants")]
    pub descendants: ColoredList,
    /// How to display that there is no bookmarks to show (none on parent,
    /// current or descendants commits).
    #[serde(default = "JujutsuBookmarkConfig::default_none")]
    pub none: ColoredText,
    /// How to display deleted pending bookmarks to be either pushed to be
    /// fully deleted or locally forget.
    #[serde(default = "JujutsuBookmarkConfig::default_deleted")]
    pub deleted: ColoredList,
}

#[allow(clippy::missing_docs_in_private_items)]
impl JujutsuBookmarkConfig {
    fn default_local() -> Color {
        Color::from(57)
    }

    fn default_untracked() -> RemoteReferenceConfig {
        RemoteReferenceConfig {
            name: Color::magenta(),
            remote: Color::from(125),
        }
    }

    fn default_tracked() -> Color {
        Color::bright_magenta()
    }

    fn default_parent() -> ColoredList {
        ColoredList::new("󰫍", "🞍", Color::yellow())
    }

    fn default_current() -> ColoredList {
        ColoredList::new("󰫍", "🞍", Color::bright_blue())
    }

    fn default_descendants() -> ColoredList {
        ColoredList::new("󰫎", "🞍", Color::bright_blue())
    }

    fn default_none() -> ColoredText {
        ColoredText::new("󰫌", Color::bright_black())
    }

    fn default_deleted() -> ColoredList {
        ColoredList::new("󰠙", "🞍", Color::ansi_color(166))
    }
}

impl Default for JujutsuBookmarkConfig {
    fn default() -> Self {
        Self {
            local: Self::default_local(),
            untracked: Self::default_untracked(),
            tracked: Self::default_tracked(),
            parent: Self::default_parent(),
            current: Self::default_current(),
            descendants: Self::default_descendants(),
            none: Self::default_none(),
            deleted: Self::default_deleted(),
        }
    }
}

/// Configuration for the Jujutsu tags prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct JujutsuTagConfig {
    /// Representation for the list of tags.
    #[serde(default = "JujutsuTagConfig::default_repr")]
    pub repr: ColoredList,
    /// Color to use to color the name of tags which exist remotely.
    #[serde(default = "JujutsuTagConfig::default_remote")]
    pub remote: RemoteReferenceConfig,
    /// Color to use to color the name of tags which exist only locally.
    #[serde(default = "JujutsuTagConfig::default_local")]
    pub local: Color,
}

#[allow(clippy::missing_docs_in_private_items)]
impl JujutsuTagConfig {
    fn default_repr() -> ColoredList {
        ColoredList::new("", "🞍", Color::yellow())
    }
    fn default_remote() -> RemoteReferenceConfig {
        RemoteReferenceConfig {
            name: Color::yellow(),
            remote: Color::from(94),
        }
    }

    fn default_local() -> Color {
        Color::bright_yellow()
    }
}

impl Default for JujutsuTagConfig {
    fn default() -> Self {
        Self {
            repr: Self::default_repr(),
            local: Self::default_local(),
            remote: Self::default_remote(),
        }
    }
}

/// Configuration for the Jujutsu prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct JujutsuPromptConfig {
    /// Color to use to print local bookmarks
    /// Configuration for the Jujutsu bookmarks prompt.
    #[serde(default)]
    pub bookmark: JujutsuBookmarkConfig,
    /// How to display the list of tags you are at.
    #[serde(default)]
    pub tags: JujutsuTagConfig,
    /// Representation to display when the working copy (current commit) has
    /// conflicts.
    #[serde(default = "JujutsuPromptConfig::default_wc_conflict")]
    pub wc_conflict: ColoredText,
    /// Representation to display when there are commits with conflicts in the
    /// history of the repository.
    #[serde(default = "JujutsuPromptConfig::default_conflict")]
    pub conflict: ColoredText,
}

#[allow(clippy::missing_docs_in_private_items)]
impl JujutsuPromptConfig {
    fn default_wc_conflict() -> ColoredText {
        ColoredText::new("󰝧", Color::bright_red())
    }

    fn default_conflict() -> ColoredText {
        ColoredText::new("󰝧", Color::red())
    }
}

impl Default for JujutsuPromptConfig {
    fn default() -> Self {
        Self {
            bookmark: JujutsuBookmarkConfig::default(),
            tags: JujutsuTagConfig::default(),
            wc_conflict: Self::default_wc_conflict(),
            conflict: Self::default_conflict(),
        }
    }
}
