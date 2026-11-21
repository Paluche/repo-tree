//! Insert a repository existing outside the repo tree, within it.
use std::fs::canonicalize;
use std::fs::create_dir_all;
use std::fs::rename;

use clap::Args;
use clap_complete::ArgValueCompleter;
use clap_complete::PathCompleter;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::repository::remove_empty_parents;
use crate::tree_space::ForceTreeSpace;
use crate::tree_space::TreeSpace;
use crate::ui::Ui;
use crate::ui_error;

/// Clone a repository within the repo tree.
#[derive(Args)]
pub struct InsertArgs {
    /// Path to the repository to insert.
    #[arg(add=ArgValueCompleter::new(PathCompleter::dir()))]
    path: String,
    /// If the repository to insert, has a remote and doubt subsist as if is an
    /// archived or not repository tree-space.
    #[arg(long, short)]
    force_tree: Option<ForceTreeSpace>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Refresh the repo tree cache based on the `refresh_cache` boolean value.
fn refresh_cache(ui: &Ui<'_>, config: &Config, refresh_cache: bool) {
    if refresh_cache {
        RepoTree::load(ui, config, true);
    }
}

/// Execute the `rt insert` command.
pub async fn run(ui: &mut Ui<'_>, config: &Config, args: InsertArgs) -> i32 {
    let path = match canonicalize(&args.path) {
        Ok(path) => path,
        Err(err) => {
            ui_error!(ui, "Invalid path {}: {err}", args.path);
            return 1;
        }
    };
    let repository = match Repository::discover_silent(config, &path) {
        Ok(r) => r,
        Err(err) => {
            ui.error(err);
            return 1;
        }
    };

    let workspace = repository.get_latest_workspace();

    let expected_root = match repository
        .expected_root(ui, config, workspace, args.force_tree.into())
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => {
            ui.error(
                "Repository is a submodule, cannot insert it into the repo \
                 tree.",
            );
            return 1;
        }
        Err(err) => {
            ui.error(err);
            return 1;
        }
    };

    let root = workspace.path();
    if root == &expected_root {
        ui.info("Repository already at the correct location");
        refresh_cache(ui, config, args.refresh_cache);
        return 0;
    }

    if let Some(parent) = expected_root.parent()
        && let Err(err) = create_dir_all(parent)
    {
        ui_error!(ui, "Cannot create {}: {err}", parent.display());
        refresh_cache(ui, config, args.refresh_cache);
        return 1;
    }

    if let Err(err) = rename(root, &expected_root) {
        ui_error!(
            ui,
            "Cannot move {} to {}: {err}",
            root.display(),
            expected_root.display()
        );
        refresh_cache(ui, config, args.refresh_cache);
        return 1;
    }
    println!("{} moved to {}", root.display(), expected_root.display());

    // Directories outside the repo tree belong to the user, only clean up
    // the ones of the repo tree, without removing its tree-spaces.
    let stop = TreeSpace::from_path(config, root).map_or_else(
        || config.root.clone(),
        |tree_space| config.root.join(tree_space.category(config).dir_name()),
    );
    match remove_empty_parents(root, &stop) {
        Ok(removed) => {
            for dir in removed {
                println!("\"{}\" removed", dir.display());
            }
        }
        Err(err) => ui_error!(
            ui,
            "Cannot remove the empty parents of {}: {err}",
            root.display()
        ),
    }

    // Repo tree changed, refresh the cache.
    refresh_cache(ui, config, true);

    0
}
