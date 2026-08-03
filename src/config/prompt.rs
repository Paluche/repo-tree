//! Configuration to customize the prompt display.

use serde::Deserialize;
use serde::Serialize;

use crate::colors::Color;
use crate::colors::ColoredList;
use crate::colors::ColoredText;

/// Configuration to representing a version control system.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VcsPromptConfig {
    /// Git Version Control System representation.
    #[serde(default = "VcsPromptConfig::default_git")]
    pub git: ColoredText,
    /// Jujutsu Version Control System representation.
    #[serde(default = "VcsPromptConfig::default_jj")]
    pub jj: ColoredText,
}

#[allow(clippy::missing_docs_in_private_items)]
impl VcsPromptConfig {
    fn default_git() -> ColoredText {
        ColoredText::new("󰊢", 166)
    }

    fn default_jj() -> ColoredText {
        ColoredText::new("", colored::Color::Blue)
    }
}

impl Default for VcsPromptConfig {
    fn default() -> Self {
        Self {
            git: Self::default_git(),
            jj: Self::default_jj(),
        }
    }
}

/// How to display the upstream information.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GitUpstreamConfig {
    /// Representation to display when the upstream associated with the current
    /// branch is gone.
    #[serde(default = "GitUpstreamConfig::default_gone")]
    gone: String,
    /// Representation to display when the current branch is up-to-date with
    /// its associated upstream.
    #[serde(default = "GitUpstreamConfig::default_up_to_date")]
    up_to_date: String,
    /// Representation to display when the current branch is ahead of its
    /// associated upstream.
    #[serde(default = "GitUpstreamConfig::default_ahead")]
    ahead: String,
    /// Representation to display when the current branch is behind of its
    /// associated upstream.
    #[serde(default = "GitUpstreamConfig::default_behind")]
    behind: String,
    /// Representation to display when the current branch diverged from its
    /// associated upstream.
    #[serde(default = "GitUpstreamConfig::default_diverged")]
    diverged: String,
    /// Representation to display when the current branch has no upstream
    /// associated.
    #[serde(default = "GitUpstreamConfig::default_local")]
    local: String,
    /// Representation to display when the current HEAD is detached from any
    /// branches.
    #[serde(default = "GitUpstreamConfig::default_detached")]
    detached: String,
    /// Color to apply on the upstream representation.
    #[serde(default = "GitUpstreamConfig::default_color")]
    color: Color,
}

#[allow(clippy::missing_docs_in_private_items)]
impl GitUpstreamConfig {
    fn default_gone() -> String {
        "".to_string()
    }

    fn default_up_to_date() -> String {
        "".to_string()
    }

    fn default_ahead() -> String {
        "".to_string()
    }

    fn default_behind() -> String {
        "".to_string()
    }

    fn default_diverged() -> String {
        "".to_string()
    }

    fn default_local() -> String {
        "".to_string()
    }

    fn default_detached() -> String {
        "".to_string()
    }

    fn default_color() -> Color {
        Color::from(208)
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn new<S, C>(
        gone: S,
        up_to_date: S,
        ahead: S,
        behind: S,
        diverged: S,
        local: S,
        detached: S,
        color: C,
    ) -> Self
    where
        S: ToString,
        Color: From<C>,
    {
        Self {
            gone: gone.to_string(),
            up_to_date: up_to_date.to_string(),
            ahead: ahead.to_string(),
            behind: behind.to_string(),
            diverged: diverged.to_string(),
            local: local.to_string(),
            detached: detached.to_string(),
            color: Color::from(color),
        }
    }

    pub fn gone(&self) -> String {
        self.color.colorize(&self.gone)
    }

    pub fn up_to_date(&self) -> String {
        self.color.colorize(&self.up_to_date)
    }

    pub fn ahead(&self) -> String {
        self.color.colorize(&self.up_to_date)
    }

    pub fn behind(&self) -> String {
        self.color.colorize(&self.behind)
    }

    pub fn diverged(&self) -> String {
        self.color.colorize(&self.diverged)
    }

    pub fn detached(&self) -> String {
        self.color.colorize(&self.detached)
    }

    pub fn local(&self) -> String {
        self.color.colorize(&self.local)
    }
}

impl Default for GitUpstreamConfig {
    fn default() -> Self {
        Self {
            gone: Self::default_gone(),
            up_to_date: Self::default_up_to_date(),
            ahead: Self::default_ahead(),
            behind: Self::default_behind(),
            diverged: Self::default_diverged(),
            local: Self::default_local(),
            detached: Self::default_detached(),
            color: Self::default_color(),
        }
    }
}

/// Configuration for the Git prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GitPromptConfig {
    /// How to display the list of ongoing operations.
    #[serde(default = "GitPromptConfig::default_ongoing_operations")]
    pub ongoing_operations: ColoredList,
    /// How to display the list of branches you are at.
    #[serde(default = "GitPromptConfig::default_branches")]
    pub branches: ColoredList,
    /// How to display the list of tags you are at.
    #[serde(default = "GitPromptConfig::default_tags")]
    pub tags: ColoredList,
    /// How to display the upstream information.
    #[serde(default)]
    pub upstream: GitUpstreamConfig,
    /// How to display the fact that there are stashed changes.
    #[serde(default = "GitPromptConfig::default_stash")]
    pub stash: ColoredText,
}

#[allow(clippy::missing_docs_in_private_items)]
impl GitPromptConfig {
    fn default_ongoing_operations() -> ColoredList {
        ColoredList::new("⛏", "🞍", colored::Color::Red)
    }

    fn default_branches() -> ColoredList {
        ColoredList::new("󰫍", "🞍", colored::Color::Blue)
    }

    fn default_tags() -> ColoredList {
        ColoredList::new("", "🞍", colored::Color::Yellow)
    }

    fn default_stash() -> ColoredText {
        ColoredText::new("", colored::Color::White)
    }
}

impl Default for GitPromptConfig {
    fn default() -> Self {
        Self {
            ongoing_operations: Self::default_ongoing_operations(),
            branches: Self::default_branches(),
            tags: Self::default_tags(),
            upstream: GitUpstreamConfig::default(),
            stash: Self::default_stash(),
        }
    }
}

/// Configuration for the Jujutsu bookmarks prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct JujutsuBookmarkConfig {
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
}

#[allow(clippy::missing_docs_in_private_items)]
impl JujutsuBookmarkConfig {
    fn default_parent() -> ColoredList {
        ColoredList::new("󰫍", "🞍", colored::Color::Yellow)
    }

    fn default_current() -> ColoredList {
        ColoredList::new("󰫍", "🞍", colored::Color::BrightBlue)
    }

    fn default_descendants() -> ColoredList {
        ColoredList::new("󰫎", "🞍", colored::Color::BrightBlue)
    }

    fn default_none() -> ColoredText {
        ColoredText::new("󰫌", colored::Color::BrightBlack)
    }
}

impl Default for JujutsuBookmarkConfig {
    fn default() -> Self {
        Self {
            parent: Self::default_parent(),
            current: Self::default_current(),
            descendants: Self::default_descendants(),
            none: Self::default_none(),
        }
    }
}

/// Configuration for the Jujutsu prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct JujutsuPromptConfig {
    /// Configuration for the Jujutsu bookmarks prompt.
    #[serde(default)]
    pub bookmark: JujutsuBookmarkConfig,
    /// How to display the list of tags you are at.
    #[serde(default = "JujutsuPromptConfig::default_tags")]
    pub tags: ColoredList,
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
        ColoredText::new("󰝧", colored::Color::BrightRed)
    }

    fn default_conflict() -> ColoredText {
        ColoredText::new("󰝧", colored::Color::Red)
    }

    fn default_tags() -> ColoredList {
        ColoredList::new("", "🞍", colored::Color::Yellow)
    }
}

impl Default for JujutsuPromptConfig {
    fn default() -> Self {
        Self {
            bookmark: JujutsuBookmarkConfig::default(),
            tags: Self::default_tags(),
            wc_conflict: Self::default_wc_conflict(),
            conflict: Self::default_conflict(),
        }
    }
}

/// Configuration to customize the prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Prefix to put in front of the prompt fields.
    #[serde(default = "PromptConfig::default_prefix")]
    pub prefix: ColoredText,
    /// String to use to separate the different fields of the prompt.
    #[serde(default = "PromptConfig::default_separator")]
    pub separator: ColoredText,
    /// Configuration to representing a version control system.
    #[serde(default)]
    pub vcs: VcsPromptConfig,
    /// Configuration relative to the Git prompt.
    #[serde(default)]
    pub git: GitPromptConfig,
    /// Configuration relative to the Jujutsu prompt.
    #[serde(default)]
    pub jj: JujutsuPromptConfig,
}

impl PromptConfig {
    /// Default value for `prefix` configuration.
    fn default_prefix() -> ColoredText {
        ColoredText::new("┣━┫", colored::Color::Cyan)
    }

    /// Default value for `separator` configuration.
    fn default_separator() -> ColoredText {
        ColoredText::new("|", colored::Color::Cyan)
    }
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            prefix: Self::default_prefix(),
            separator: Self::default_separator(),
            vcs: VcsPromptConfig::default(),
            git: GitPromptConfig::default(),
            jj: JujutsuPromptConfig::default(),
        }
    }
}
