//! Configuration for git status command.

use serde::Deserialize;
use serde::Serialize;

use crate::colors::Color;
use crate::colors::ColoredText;

/// How to display the upstream information in the git status.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GitUpstreamStatusConfig {
    /// Text to use to represent the number of commits ahead the local branch
    /// is relative to its upstream.
    #[serde(default = "GitUpstreamStatusConfig::default_ahead")]
    pub ahead: ColoredText,
    /// Text to use to represent the number of commits behind the local branch
    /// is relative to its upstream.
    #[serde(default = "GitUpstreamStatusConfig::default_behind")]
    pub behind: ColoredText,
    /// Color to use to colorize the name of the upstream.
    #[serde(default = "GitUpstreamStatusConfig::default_name")]
    pub name: Color,
}

#[allow(clippy::missing_docs_in_private_items)]
impl GitUpstreamStatusConfig {
    fn default_ahead() -> ColoredText {
        ColoredText::new("", Color::green())
    }

    fn default_behind() -> ColoredText {
        ColoredText::new("", Color::red())
    }

    fn default_name() -> Color {
        Color::cyan()
    }
}

impl Default for GitUpstreamStatusConfig {
    fn default() -> Self {
        Self {
            ahead: Self::default_ahead(),
            behind: Self::default_behind(),
            name: Self::default_name(),
        }
    }
}

/// How to display the git status entries.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GitStatusEntryConfig {
    /// Entry is not modified.
    #[serde(default = "GitStatusEntryConfig::default_unmodified")]
    pub unmodified: char,
    /// Entry is modified.
    #[serde(default = "GitStatusEntryConfig::default_modified")]
    pub modified: char,
    /// Entry has changed file type.
    #[serde(default = "GitStatusEntryConfig::default_file_type_changed")]
    pub file_type_changed: char,
    /// Entry has been added.
    #[serde(default = "GitStatusEntryConfig::default_added")]
    pub added: char,
    /// Entry has been deleted.
    #[serde(default = "GitStatusEntryConfig::default_deleted")]
    pub deleted: char,
    /// Entry has been renamed.
    #[serde(default = "GitStatusEntryConfig::default_renamed")]
    pub renamed: char,
    /// Entry has been copied.
    #[serde(default = "GitStatusEntryConfig::default_copied")]
    pub copied: char,
    /// Entry has been updated.
    #[serde(default = "GitStatusEntryConfig::default_updated")]
    pub updated: char,
    /// Entry is untracked.
    #[serde(default = "GitStatusEntryConfig::default_untracked")]
    pub untracked: char,
    /// Entry is ignored.
    #[serde(default = "GitStatusEntryConfig::default_ignored")]
    pub ignored: char,
}

#[allow(clippy::missing_docs_in_private_items)]
impl GitStatusEntryConfig {
    fn default_unmodified() -> char {
        ' '
    }

    fn default_modified() -> char {
        'M'
    }

    fn default_file_type_changed() -> char {
        'T'
    }

    fn default_added() -> char {
        'A'
    }

    fn default_deleted() -> char {
        'D'
    }

    fn default_renamed() -> char {
        'R'
    }

    fn default_copied() -> char {
        'C'
    }

    fn default_updated() -> char {
        'U'
    }

    fn default_untracked() -> char {
        '?'
    }

    fn default_ignored() -> char {
        '!'
    }
}

impl Default for GitStatusEntryConfig {
    fn default() -> Self {
        Self {
            unmodified: Self::default_unmodified(),
            modified: Self::default_modified(),
            file_type_changed: Self::default_file_type_changed(),
            added: Self::default_added(),
            deleted: Self::default_deleted(),
            renamed: Self::default_renamed(),
            copied: Self::default_copied(),
            updated: Self::default_updated(),
            untracked: Self::default_untracked(),
            ignored: Self::default_ignored(),
        }
    }
}

/// Configuration for the `rt git status` command.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GitStatusCommandConfig {
    /// How to display the upstream information in the git status.
    pub upstream: GitUpstreamStatusConfig,
    /// Character used to represent the different entry status possible.
    #[serde(default)]
    pub entry: GitStatusEntryConfig,
    /// Color used for coloring entries which are non modified, untracked or
    /// ignored.
    #[serde(default = "GitStatusCommandConfig::default_unimportant")]
    pub unimportant: Color,
    /// Color used for coloring updated entries.
    #[serde(default = "GitStatusCommandConfig::default_updated")]
    pub updated: Color,
    /// Color used for coloring staged entries.
    #[serde(default = "GitStatusCommandConfig::default_staged")]
    pub staged: Color,
    /// Color used for coloring unstaged entries.
    #[serde(default = "GitStatusCommandConfig::default_unstaged")]
    pub unstaged: Color,
    /// Color used for coloring submodule status.
    #[serde(default = "GitStatusCommandConfig::default_submodule")]
    pub submodule: Color,
}

#[allow(clippy::missing_docs_in_private_items)]
impl GitStatusCommandConfig {
    fn default_unimportant() -> Color {
        Color::white()
    }

    fn default_updated() -> Color {
        Color::red()
    }

    fn default_staged() -> Color {
        Color::green()
    }

    fn default_unstaged() -> Color {
        Color::red()
    }

    fn default_submodule() -> Color {
        Color::blue()
    }
}

impl Default for GitStatusCommandConfig {
    fn default() -> Self {
        Self {
            upstream: GitUpstreamStatusConfig::default(),
            entry: GitStatusEntryConfig::default(),
            unimportant: Self::default_unimportant(),
            updated: Self::default_updated(),
            staged: Self::default_staged(),
            unstaged: Self::default_unstaged(),
            submodule: Self::default_submodule(),
        }
    }
}
