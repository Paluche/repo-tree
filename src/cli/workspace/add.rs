//! Create a workspace for a repository in a specified workspace tree-space.

use clap::ArgAction;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;

use crate::config::Config;
use crate::resolve::resolve_completer;
use crate::tree::TreeSpace;


/// Create a workspace for a specified repository into a specified tree-space
/// workspace.
#[derive(Args)]
pub struct WorkspaceAddArgs{
    /// Path to repository to create a workspace for.
    #[arg(add=ArgValueCompleter::new(resolve_completer))]
    repo_id: Option<String>,

    #[arg(long, short, default_value_t=TreeSpace::Agent, add=ArgValueCompleter::new(TreeSpace::workspace_tree_space_completer))]
    tree_space: TreeSpace,

    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

pub fn run(config: &Config, args: WorkspaceAddArgs) -> i32 {
    2
}
