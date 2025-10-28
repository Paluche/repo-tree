//! Action to clean the workspace.
//! Replace the repositories where they belong to.

use crate::{Config, Repository, UrlParser, get_work_dir, load_workspace};
use std::fs::{create_dir_all, remove_dir, rename};

pub fn clean(dry_run: bool) -> i32 {
    let work_dir = get_work_dir();
    let config = Config::default();
    let url_parser = UrlParser::new(&config);
    let repositories = load_workspace(&url_parser)
        .0
        .into_iter()
        .filter(|r| r.expected_root(&work_dir).is_some_and(|p| p != r.root))
        .collect::<Vec<Repository>>();

    let mut ret = 0;

    if repositories.is_empty() {
        println!("All repositories are where they belong");
    } else {
        println!("Repositories to move:");
        for repository in repositories {
            let expected_root = repository.expected_root(&work_dir).unwrap();
            println!(
                "- {}: {} => {}",
                repository.id.name,
                repository.root.display(),
                expected_root.display(),
            );

            if dry_run {
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
        let empty_dirs = load_workspace(&url_parser).1;

        if empty_dirs.is_empty() {
            if first {
                println!("No empty directories to remove");
            }
            break;
        }
        first = false;

        for empty_dir in empty_dirs {
            println!("Removing empty directory: {}", empty_dir.display());
            if !dry_run && let Err(err) = remove_dir(empty_dir) {
                eprintln!("{err}");
                ret = 1;
                break;
            }
        }
    }

    ret
}
