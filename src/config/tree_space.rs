//! Configuration related to the tree-spaces.

use serde::Deserialize;
use serde::Serialize;

use super::TreeCategory;
use crate::colors::ColoredText;

/// Configuration for the different tree spaces.
#[derive(Serialize, Deserialize, Default)]
pub struct TreeSpaceConfig {
    /// Configuration for active / in development repositories tree-space.
    #[serde(default)]
    pub dev: DevTreeSpace,
    /// Configuration for local only repositories.
    #[serde(default)]
    pub local: LocalTreeSpace,
    /// Configuration for archived repositories tree-space.
    #[serde(default)]
    pub archive: ArchiveTreeSpace,
    /// Configuration for agents repositories' workspaces in the tree-space.
    #[serde(default)]
    pub agents: AgentsTreeSpace,
}

/// Configuration for the dev tree-space, the default tree-space for
/// repositories which has remote and are non-archived.
#[derive(Serialize, Deserialize, Debug)]
pub struct DevTreeSpace {
    /// Tree category information for the tree-space.
    #[serde(flatten)]
    pub category: TreeCategory,
}

impl Default for DevTreeSpace {
    fn default() -> Self {
        Self {
            category: TreeCategory::new(
                "dev".to_string(),
                None,
                ColoredText::new("", colored::Color::Blue),
            ),
        }
    }
}

/// Configuration for the tree-space that will contain the repositories which
/// has no associated remote, therefore existing only locally.
#[derive(Serialize, Deserialize)]
pub struct LocalTreeSpace {
    /// Tree category information for the tree-space.
    #[serde(flatten)]
    pub category: TreeCategory,
}

impl Default for LocalTreeSpace {
    fn default() -> Self {
        Self {
            category: TreeCategory::new(
                "local".to_string(),
                None,
                ColoredText::new("󰋊", colored::Color::White),
            ),
        }
    }
}

/// Configuration for the tree-space that will contain archived repositories.
#[derive(Serialize, Deserialize)]
pub struct ArchiveTreeSpace {
    /// Tree category information for the tree-space.
    #[serde(flatten)]
    pub category: TreeCategory,
}

impl Default for ArchiveTreeSpace {
    fn default() -> Self {
        Self {
            category: TreeCategory::new(
                "archive".to_string(),
                None,
                ColoredText::new("󰀼", colored::Color::Yellow),
            ),
        }
    }
}

/// Configuration for the tree-space that will contain the repositories'
/// workspaces where agents will work to dissociate them from the workspaces
/// where the human develop.
#[derive(Serialize, Deserialize)]
pub struct AgentsTreeSpace {
    /// Tree category information for the tree-space.
    #[serde(flatten)]
    pub category: TreeCategory,
}

impl Default for AgentsTreeSpace {
    fn default() -> Self {
        Self {
            category: TreeCategory::new(
                "agents".to_string(),
                None,
                ColoredText::new("󰚩", colored::Color::BrightYellow),
            ),
        }
    }
}
