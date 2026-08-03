//! Configuration of allowed repository locations outside the repository tree.
use std::hash::Hash;
use std::path::Path;

use globset::Glob;
use serde::Deserialize;
use serde::Serialize;

/// Configuration regarding allowed repository locations.
#[derive(Serialize, Deserialize, Hash, PartialEq)]
pub struct RepositoryLocation {
    /// List of glob patterns, any repositories path matching one of the
    /// defined pattern will be allowed to live outside the repo tree. No
    /// warning message will be printed when the prompt run.
    #[serde(default = "RepositoryLocation::default_ignore")]
    pub ignore: Vec<Glob>,
    /// List of glob pattern to extend the ignore configuration value.
    pub extend_ignore: Vec<Glob>,
}

impl RepositoryLocation {
    /// Default value for the ignore value of the RepositoryLocation struct.
    fn default_ignore() -> Vec<Glob> {
        ["/tmp/**", "**/.*/**"]
            .into_iter()
            .map(|v| {
                Glob::new(v)
                    .expect("Hardcoded values should be valid glob patterns.")
            })
            .collect()
    }

    /// Find out if a repository located at the specified path should be
    /// ignored as being a badly located repository due to not being within the
    /// repo tree.
    pub fn should_be_ignored(&self, path: &Path) -> bool {
        path.to_str()
            .map(|path| {
                self.ignore
                    .iter()
                    .chain(self.extend_ignore.iter())
                    .any(|glob| glob.compile_matcher().is_match(path))
            })
            .unwrap_or(false)
    }
}

impl Default for RepositoryLocation {
    fn default() -> Self {
        Self {
            ignore: Self::default_ignore(),
            extend_ignore: Vec::new(),
        }
    }
}
