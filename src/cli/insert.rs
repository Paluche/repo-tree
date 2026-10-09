//! Insert a repository existing outside the repo tree, within it.
use std::fs::create_dir_all;
use std::fs::remove_dir;
use std::fs::rename;
use std::path::PathBuf;

use clap::Args;
use clap_complete::ArgValueCompleter;
use clap_complete::PathCompleter;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::tree_space::ForceTreeSpace;
use crate::ui::Ui;

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

/// Refresh the repo tree cache based on the refresh_cache boolean value.
fn refresh_cache(config: &Config, ui: &Ui<'_>, refresh_cache: bool) {
    if refresh_cache {
        RepoTree::load(config, ui, true);
    }
}

/// Execute the `rt insert` command.
pub async fn run(config: &Config, ui: &mut Ui<'_>, args: InsertArgs) -> i32 {
    let repository =
        match Repository::discover_silent(config, &PathBuf::from(args.path)) {
            Ok(r) => r,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        };

    let workspace = repository.get_latest_workspace();

    let expected_root = match repository
        .expected_root(
            config,
            ui,
            repository.get_latest_workspace(),
            args.force_tree.into(),
        )
        .await
    {
        Ok(value) => match value {
            Some(value) => value,
            None => {
                eprintln!(
                    "Repository is a submodule, cannot insert it into the \
                     repo tree."
                );
                return 1;
            }
        },
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

    let root = workspace.path();
    if root == &expected_root {
        eprintln!("Repository already at the correct location");
        refresh_cache(config, ui, args.refresh_cache);
        return 0;
    }

    let parent = expected_root.parent().unwrap();

    if !parent.exists()
        && let Err(err) = create_dir_all(parent)
    {
        ui.error(err);
        refresh_cache(config, ui, args.refresh_cache);
        return 1;
    }

    if let Err(err) = rename(root, &expected_root) {
        ui.error(err);
        refresh_cache(config, ui, args.refresh_cache);
        return 1;
    }
    println!("{} moved to {}", root.display(), expected_root.display());

    let mut current = root.as_path();
    let mut removed = false;
    while let Some(next) = current.parent() {
        if next
            .read_dir()
            .expect("read dir call failed")
            .flatten()
            .count()
            != 0
        {
            break;
        }

        if let Err(err) = remove_dir(next) {
            eprintln!("{err}");
            break;
        }
        current = next;
        removed = true;
    }

    if removed {
        println!("\"{}\" removed", current.display());
    }

    // Repo tree changed, refresh the cache.
    refresh_cache(config, ui, true);

    0
}
