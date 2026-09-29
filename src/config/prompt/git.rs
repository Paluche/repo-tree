//! Configuration for the Git prompt.

use serde::Deserialize;
use serde::Serialize;

use crate::colors::Color;
use crate::colors::ColoredList;
use crate::colors::ColoredText;

/// Default colored text to represent the Git version control system.
pub fn default_vcs_prompt() -> ColoredText {
    ColoredText::new("󰊢", 166)
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
        Color::ansi_color(208)
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
        ColoredList::new("⛏", "🞍", Color::red())
    }

    fn default_branches() -> ColoredList {
        ColoredList::new("󰫍", "🞍", Color::blue())
    }

    fn default_tags() -> ColoredList {
        ColoredList::new("", "🞍", Color::yellow())
    }

    fn default_stash() -> ColoredText {
        ColoredText::new("", Color::white())
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
