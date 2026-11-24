//! Configuration related to the tree-spaces.

use serde::Deserialize;
use serde::Serialize;

use super::TreeCategory;
use super::tree_category::PartialTreeCategory;
use crate::colors::Color;
use crate::colors::ColoredText;

/// Configuration for the different tree spaces.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
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
    /// Configuration for agent repositories' workspaces in the tree-space.
    #[serde(default)]
    pub agent: AgentTreeSpace,
}

/// Configuration for the dev tree-space, the default tree-space for
/// repositories which has remote and are non-archived.
#[derive(Serialize, Deserialize, Debug)]
#[serde(from = "PartialTreeCategory")]
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
                ColoredText::new("", Color::blue()),
            ),
        }
    }
}

/// Configuration for the tree-space that will contain the repositories which
/// has no associated remote, therefore existing only locally.
#[derive(Serialize, Deserialize)]
#[serde(from = "PartialTreeCategory")]
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
                ColoredText::new("󰋊", Color::white()),
            ),
        }
    }
}

/// Configuration for the tree-space that will contain archived repositories.
#[derive(Serialize, Deserialize)]
#[serde(from = "PartialTreeCategory")]
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
                ColoredText::new("󰀼", Color::yellow()),
            ),
        }
    }
}

/// Configuration for the tree-space that will contain the repositories'
/// workspaces where agents will work to dissociate them from the workspaces
/// where the human develop.
#[derive(Serialize, Deserialize)]
#[serde(from = "PartialTreeCategory")]
pub struct AgentTreeSpace {
    /// Tree category information for the tree-space.
    #[serde(flatten)]
    pub category: TreeCategory,
}

impl Default for AgentTreeSpace {
    fn default() -> Self {
        Self {
            category: TreeCategory::new(
                "agent".to_string(),
                None,
                ColoredText::new("󰚩", colored::Color::BrightYellow),
            ),
        }
    }
}

/// Implement the deserialization of a tree-space configuration, as an
/// override of its default configuration.
macro_rules! impl_from_partial {
    ($($tree_space:ident),*) => {
        $(
            impl From<PartialTreeCategory> for $tree_space {
                fn from(partial: PartialTreeCategory) -> Self {
                    Self {
                        category: partial.apply(Self::default().category),
                    }
                }
            }
        )*
    };
}

impl_from_partial!(
    DevTreeSpace,
    LocalTreeSpace,
    ArchiveTreeSpace,
    AgentTreeSpace
);
