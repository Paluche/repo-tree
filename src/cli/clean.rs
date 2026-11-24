//! Action to clean the repo_tree.
//! Move the repositories where they belong to and delete empty directories.

use std::fs::create_dir_all;
use std::fs::remove_dir;
use std::fs::rename;
use std::path::PathBuf;

use clap::Args;
use pollster::FutureExt;

use crate::config::Config;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::repository::Workspace;
use crate::ui::Ui;
use crate::ui_error;

/// Clean the repo_tree. Move the repositories where they belong and remove
/// empty directories.
#[derive(Args)]
pub struct CleanArgs {
    /// Do not perform any change on the repo_tree. Simply print what would be
    /// done.
    #[arg(short, long)]
    dry_run: bool,
}

/// Execute the `rt clean` command.
pub async fn run(ui: &mut Ui<'_>, config: &Config, args: CleanArgs) -> i32 {
    // Do not use the cache, assure we have an up-to-date list of repositories
    // before doing any action that will modify the directories.
    let repo_tree = RepoTree::load_silent(ui, config, true);
    let repos_to_move: Vec<(&Repository, &Workspace, PathBuf)> = repo_tree
        .workspace_iter()
        .filter_map(|(r, w)| {
            match r
                .expected_root(ui, config, w, ExpectedTreeStrategy::Exact)
                .block_on()
            {
                Ok(v) => v.and_then(|p| (&p != w.path()).then_some((r, w, p))),
                Err(err) => {
                    ui.error(err);
                    None
                }
            }
        })
        .collect();

    let mut ret = 0;

    if repos_to_move.is_empty() {
        println!("All repositories are where they belong");
    } else {
        println!("Repositories to move:");
        for (repository, workspace, expected_root) in repos_to_move {
            println!(
                "- {}: {} => {}",
                repository.id.name,
                workspace.path().display(),
                expected_root.display(),
            );

            if args.dry_run {
                continue;
            }

            if let Some(parent) = expected_root.parent()
                && let Err(err) = create_dir_all(parent)
            {
                ui.error(err);
                ret = 1;
                continue;
            }

            if let Err(err) = rename(workspace.path(), &expected_root) {
                ui_error!(
                    ui,
                    "Cannot move {} to {}: {err}",
                    workspace.path().display(),
                    expected_root.display()
                );
                ret = 1;
            }
        }
    }

    // Removing empty directories may leave their parent empty, search again
    // until there is no more empty directories, or a removal failed.
    let mut first = true;
    loop {
        // Force the cache to be refreshed at the same time as loading the empty
        // directories.
        let empty_dirs =
            RepoTree::load_silent_with_empty_dirs(ui, config, true)
                .1
                .unwrap_or_default();

        if empty_dirs.is_empty() {
            if first {
                println!("No empty directories to remove");
            }
            break;
        }
        first = false;

        if args.dry_run {
            for empty_dir in empty_dirs {
                println!("Empty directory to remove: {}", empty_dir.display());
            }
            break;
        }

        let mut failed = false;
        for empty_dir in empty_dirs {
            println!("Removing empty directory: {}", empty_dir.display());
            if let Err(err) = remove_dir(&empty_dir) {
                ui_error!(ui, "Cannot remove {}: {err}", empty_dir.display());
                failed = true;
            }
        }
        if failed {
            ret = 1;
            break;
        }
    }

    ret
}
