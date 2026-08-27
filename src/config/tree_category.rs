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

    #[cfg(test)]
    /// Get the raw `dir_name` configuration value.
    pub fn raw_dir_name(&self) -> Option<&str> {
        self.dir_name.as_deref()
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
