//! Action to clean the repo_tree.
//! Move the repositories where they belong to and delete empty directories.

use std::fs::create_dir_all;
use std::fs::remove_dir;
use std::fs::rename;
use std::path::PathBuf;

use clap::Args;

use crate::config::Config;
use crate::repository::Repository;
use crate::tree::RepoTree;

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
pub fn run(config: &Config, args: CleanArgs) -> i32 {
    // Do not use the cache, assure we have an up-to-date list of repositories
    // before doing any action that will modify the directories.
    let repo_tree = RepoTree::load_silent(config, true);
    let repos_to_move: Vec<(&Repository, PathBuf)> = repo_tree
        .iter()
        .filter_map(|r| match r.expected_root(config) {
            Ok(v) => v.and_then(|p| (p != r.root).then_some((r, p))),
            Err(err) => {
                eprintln!("{err}");
                None
            }
        })
        .collect();

    let mut ret = 0;

    if repos_to_move.is_empty() {
        println!("All repositories are where they belong");
    } else {
        println!("Repositories to move:");
        for (repository, expected_root) in repos_to_move {
            println!(
                "- {}: {} => {}",
                repository.id.name,
                repository.root.display(),
                expected_root.display(),
            );

            if args.dry_run {
                continue;
            }

            let parent = expected_root.parent().unwrap();

            if !parent.exists()
                && let Err(err) = create_dir_all(parent)
            {
                eprintln!("{err}");
                ret = 1;
            }

            if let Err(err) = rename(&repository.root, expected_root) {
                eprintln!("{err}");
                ret = 1;
            }
        }
    }

    let mut first = true;
    loop {
        // Force the cache to be refreshed at the same time as loading the empty
        // directories.
        let (_, Some(empty_dirs)) =
            RepoTree::load_silent_with_empty_dirs(config, true)
        else {
            panic!(
                "Cache forced to be refreshed so empty_dirs should be \
                 available"
            );
        };

        if empty_dirs.is_empty() {
            if first {
                println!("No empty directories to remove");
            }
            break;
        }
        first = false;

        for empty_dir in empty_dirs {
            println!("Removing empty directory: {}", empty_dir.display());
            if !args.dry_run
                && let Err(err) = remove_dir(empty_dir)
            {
                eprintln!("{err}");
                ret = 1;
                break;
            }
        }
    }

    ret
}
