//! Action to clean the repo_tree.
//! Move the repositories where they belong to and delete empty directories.

use std::fs::create_dir_all;
use std::fs::remove_dir;
use std::fs::rename;

use clap::Args;

use crate::config::Config;
use crate::repository::Repositories;
use crate::repository::Repository;

/// Clean the repo_tree. Move the repositories where they belong and remove
/// empty directories.
#[derive(Args, Debug, PartialEq)]
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
    let repositories = Repositories::load_silent(config, true);
    let repos_to_move: Vec<&Repository> = repositories
        .iter()
        .filter(|r| match r.expected_root(config) {
            Ok(v) => v.is_some_and(|p| p != r.root),
            Err(err) => {
                eprintln!("{err}");
                false
            }
        })
        .collect();

    let mut ret = 0;

    if repos_to_move.is_empty() {
        println!("All repositories are where they belong");
    } else {
        println!("Repositories to move:");
        for repository in repos_to_move {
            let expected_root =
                repository.expected_root(config).unwrap().unwrap();
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
            Repositories::load_silent_with_empty_dirs(config, true)
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
