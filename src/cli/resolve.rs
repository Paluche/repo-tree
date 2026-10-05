//! Action to resolve the path to a repository from its name or alias.

use clap::Args;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::resolve::ResolveFilter;
use crate::resolve::resolve;
use crate::resolve::resolve_completer;
use crate::tree_space::TreeSpace;
use crate::tree_space::TreeSpaceKind;
use crate::ui::Ui;

/// Resolve the name of a repository into its path.
#[derive(Args)]
pub struct ResolveArgs {
    /// Repository identifier to resolve into the actual path within the
    /// repo_tree.
    #[arg(add=resolve_completer())]
    repo_id: Option<String>,
    /// Precise the tree-space from which you want the repository.
    #[arg(short, long, conflicts_with="kind", add=TreeSpace::completer())]
    tree: Option<TreeSpace>,
    /// Precise the tree-space from which you want the repository.
    #[arg(short, long, conflicts_with="tree", add=TreeSpaceKind::completer())]
    kind: Option<TreeSpaceKind>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt resolve` command.
pub fn run(config: &Config, ui: &Ui<'_>, args: ResolveArgs) -> i32 {
    let repo_tree = RepoTree::load(config, ui, args.refresh_cache);
    let Ok(filter) = ResolveFilter::from_cli_args(args.tree, args.kind) else {
        panic!(
            "Arguments should have been mutually conflicting and managed by \
             clap."
        );
    };

    if let Some((_, workspace)) =
        match resolve(config, ui, &repo_tree, args.repo_id, filter) {
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
