//! Action to resolve the path to a repository from its name or alias.

use clap::Args;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::resolve::resolve;
use crate::resolve::resolve_completer;
use crate::tree_space::TreeSpace;

/// Resolve the name of a repository into its path.
#[derive(Args)]
pub struct ResolveArgs {
    /// Repository identifier to resolve into the actual path within the
    /// repo_tree.
    #[arg(add=resolve_completer())]
    repo_id: Option<String>,
    /// Precise the tree-space from which you want the repository.
    #[arg(short, long, add=TreeSpace::completer())]
    tree: Option<TreeSpace>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt resolve` command.
pub fn run(config: &Config, args: ResolveArgs) -> i32 {
    let repo_tree = RepoTree::load(config, args.refresh_cache);
    if let Some((_, workspace)) =
        match resolve(config, &repo_tree, args.repo_id, args.tree.as_ref()) {
            Ok(r) => r,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        }
    {
        println!("{}", workspace.path().display());
        0
    } else {
        2
    }
}
