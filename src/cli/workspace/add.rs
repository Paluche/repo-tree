//! Create a workspace for a repository in a specified workspace tree-space.

use clap::Args;

use super::super::get_current_repo_or_main;
use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::resolve::resolve_completer;
use crate::tree_space::WorkspaceTreeSpace;
use crate::ui::Ui;

/// Create a workspace for a specified repository into a specified tree-space
/// workspace.
#[derive(Args)]
pub struct WorkspaceAddArgs {
    /// Identifier of the repository to create a workspace for.
    #[arg(add=resolve_completer())]
    repo_id: Option<String>,

    /// Specify in which workspace tree-space to create the workspace.
    #[arg(long, short, default_value_t=WorkspaceTreeSpace::Agent, add=WorkspaceTreeSpace::completer())]
    workspace: WorkspaceTreeSpace,

    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt workspace add` action.
pub fn run(config: &Config, ui: &Ui, args: WorkspaceAddArgs) -> i32 {
    let repo_tree = RepoTree::load(config, ui, args.refresh_cache);

    let repository = match get_current_repo_or_main(
        config,
        ui,
        &repo_tree,
        args.repo_id,
    ) {
        Ok(Some((r, _w))) => r,
        Ok(None) => {
            ui.error(
                "Current directory is not a repository or one within the repo-tree."
            );
            return 2;
        }
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

    match repository.add_workspace_in_tree(ui, config, &args.workspace.into()) {
        Ok(_) => (),
        Err(err) => {
            ui.error(err);
            return 1;
        }
    }

    // Refresh the cache.
    RepoTree::load(config, ui, true);

    0
}
