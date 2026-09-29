//! Configuration for specific repo-tree commands.
mod clone;
mod git_status;
mod resolve;
mod todo;

pub use clone::CloneCommandConfig;
pub use git_status::GitStatusCommandConfig;
pub use resolve::ResolveCommandConfig;
use serde::Deserialize;
use serde::Serialize;
pub use todo::TodoCommandConfig;

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
    /// Configuration for `rt git status`.
    #[serde(default)]
    pub git_status: GitStatusCommandConfig,
}
