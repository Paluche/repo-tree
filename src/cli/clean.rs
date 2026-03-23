//! Action to clean the repo_tree.
//! Replace the repositories where they belong to.

use std::fs::{create_dir_all, remove_dir, rename};

use clap::Args;

use crate::{Config, Repository, load_empty_dirs, load_repositories_silent};

/// Clean the repo_tree. Move the repositories where they belong and remove
/// empty directories.
#[derive(Args, Debug, PartialEq)]
pub struct CleanArgs {
    /// Do not perform any change on the repo_tree. Simply print what would be
    /// done.
    #[arg(short, long)]
    dry_run: bool,
}

pub fn run(args: CleanArgs) -> i32 {
    let config = Config::default();
    let repositories = load_repositories_silent(&config)
        .into_iter()
        .filter(|r| r.expected_root(&config).is_some_and(|p| p != r.root))
        .collect::<Vec<Repository>>();

    let mut ret = 0;

    if repositories.is_empty() {
        println!("All repositories are where they belong");
    } else {
        println!("Repositories to move:");
        for repository in repositories {
            let expected_root = repository.expected_root(&config).unwrap();
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

            if let Err(err) = rename(repository.root, expected_root) {
                eprintln!("{err}");
                ret = 1;
            }
        }
    }

    let mut first = true;
    loop {
        let empty_dirs = load_empty_dirs(&config);

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
