//! Configuration for specific repo-tree commands.

use std::collections::BTreeMap;

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

/// Configuration for the `rt resolve` command.
#[derive(Serialize, Deserialize, Default)]
pub struct ResolveCommandConfig {
    /// Resolution aliases.
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
}

/// Configuration for the `rt todo` command.
#[derive(Serialize, Deserialize, Default)]
pub struct TodoCommandConfig {
    /// List of ID of repositories to be ignored by the command.
    #[serde(default)]
    pub ignore: Vec<String>,
}

/// Configuration for `rt` commands.
#[derive(Serialize, Deserialize, Default)]
pub struct CommandConfig {
    /// Configuration for `rt clone`.
    #[serde(default)]
    pub clone: CloneCommandConfig,
    /// Configuration for `rt resolve`.
    #[serde(default)]
    pub resolve: ResolveCommandConfig,
    /// Configuration for `rt todo`.
    #[serde(default)]
    pub todo: TodoCommandConfig,
}
