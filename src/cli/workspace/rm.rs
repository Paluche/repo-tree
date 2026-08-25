//! Remove a workspace repository from a specified workspace tree-space.
use clap::Args;

use super::super::get_current_repo_or_main;
use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::tree_space::TreeSpace;
use crate::resolve::resolve_completer;
use crate::tree_space::WorkspaceTreeSpace;
use crate::ui::Ui;

/// Create a workspace for a specified repository into a specified tree-space
/// workspace.
#[derive(Args)]
pub struct WorkspaceRmArgs {
    /// Path to repository to create a workspace for.
    #[arg(add=resolve_completer())]
    repo_id: Option<String>,
    /// Specify in which workspace tree-space the workspace to remove is.
    #[arg(long, short, default_value_t=WorkspaceTreeSpace::Agent, add=WorkspaceTreeSpace::completer())]
    workspace: WorkspaceTreeSpace,
    /// Force the removal of the workspace.
    #[arg(short, long)]
    force: bool,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt workspace rm` action.
pub fn run(config: &Config, ui: &Ui, args: WorkspaceRmArgs) -> i32 {
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

    let tree_space: TreeSpace =args.workspace.into();

    let Some(workspace) = repository.get_tree_workspace(ui, &tree_space) else {
        ui.error(
            format!(
                "The repository {} does not have a workspace in the tree-space {}",
                repository.id.display(config),
                tree_space.display(config),
            )
        );
        return 2;
    };

    if !args.force {
        // Ask the user for confirmation before removing the repository.
        println!(
            "Are you sure you want to remove the repository {} from the
            repo-tree? [y/N]",
            repository.id.display(config),
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() != "y" {
            println!("Aborting.");
            return 1;
        }
    }

    workspace.rm();

    // Refresh the cache.
    RepoTree::load(config, ui, true);

    0
}
