//! Configuration to customize the prompt display.
mod git;
mod jujutsu;

pub use git::GitPromptConfig;
pub use jujutsu::JujutsuBookmarkConfig;
pub use jujutsu::JujutsuPromptConfig;
pub use jujutsu::JujutsuTagConfig;
use serde::Deserialize;
use serde::Serialize;

use crate::colors::Color;
use crate::colors::ColoredText;

/// Configuration to representing a version control system.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VcsPromptConfig {
    /// Git Version Control System representation.
    #[serde(default = "git::default_vcs_prompt")]
    pub git: ColoredText,
    /// Jujutsu Version Control System representation.
    #[serde(default = "jujutsu::default_vcs_prompt")]
    pub jj: ColoredText,
}

impl Default for VcsPromptConfig {
    fn default() -> Self {
        Self {
            git: git::default_vcs_prompt(),
            jj: jujutsu::default_vcs_prompt(),
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
    /// How to colorize the ID of the repository in the prompt.
    #[serde(default = "PromptConfig::default_id")]
    pub id: Color,
    /// Configuration relative to the Git prompt.
    #[serde(default)]
    pub git: GitPromptConfig,
    /// Configuration relative to the Jujutsu prompt.
    #[serde(default)]
    pub jj: JujutsuPromptConfig,
}

#[allow(clippy::missing_docs_in_private_items)]
impl PromptConfig {
    fn default_prefix() -> ColoredText {
        ColoredText::new("┣━┫", Color::cyan())
    }

    fn default_separator() -> ColoredText {
        ColoredText::new("|", Color::cyan())
    }

    fn default_id() -> Color {
        Color::green()
    }
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            prefix: Self::default_prefix(),
            separator: Self::default_separator(),
            vcs: VcsPromptConfig::default(),
            id: Self::default_id(),
            git: GitPromptConfig::default(),
            jj: JujutsuPromptConfig::default(),
        }
    }
}
