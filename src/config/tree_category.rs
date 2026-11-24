//! Generic structure for configuration which represents a category in the
//! repo-tree. A category is a directory in the repository tree which has a
//! prompt representation associated.

use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;

use crate::colors::ColoredText;

/// Representation of a category in the repo tree.
#[derive(Serialize, Deserialize, Clone, PartialEq, Hash, Debug)]
pub struct TreeCategory {
    /// Name of the category.
    pub name: String,

    /// Name of the directory for that object in the repo tree.
    #[serde(default)]
    dir_name: Option<String>,

    /// Short representation of the category.
    #[serde(default)]
    pub repr: ColoredText,
}

impl TreeCategory {
    /// Create a new instance of TreeCategory.
    pub fn new(
        name: String,
        dir_name: Option<String>,
        repr: ColoredText,
    ) -> Self {
        Self {
            name,
            dir_name,
            repr,
        }
    }

    /// Get the directory name for that host in the repo tree.
    pub fn dir_name(&self) -> &str {
        self.dir_name.as_ref().unwrap_or(&self.name)
    }
}

/// Partial representation of a category, as written in the configuration
/// to override some fields of a default category.
#[derive(Deserialize)]
pub struct PartialTreeCategory {
    /// Name of the category.
    name: Option<String>,
    /// Name of the directory for that object in the repo tree.
    dir_name: Option<String>,
    /// Short representation of the category.
    repr: Option<ColoredText>,
}

impl PartialTreeCategory {
    /// Override the fields of `default` with the ones defined.
    pub fn apply(self, default: TreeCategory) -> TreeCategory {
        TreeCategory {
            name: self.name.unwrap_or(default.name),
            dir_name: self.dir_name.or(default.dir_name),
            repr: self.repr.unwrap_or(default.repr),
        }
    }
}

impl Display for TreeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.repr.is_empty() {
            write!(f, "{}", self.name)
        } else {
            self.repr.fmt(f)
        }
    }
}
