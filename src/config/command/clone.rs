//! Configuration for the clone command.

use serde::Deserialize;
use serde::Serialize;

use crate::version_control_system::VersionControlSystem;

/// Configuration for the `rt clone` command.
#[derive(Serialize, Deserialize, Default)]
pub struct CloneCommandConfig {
    /// Default version control system to use to clone a repository in the repo
    /// tree.
    #[serde(default)]
    pub default_vcs: VersionControlSystem,
}
