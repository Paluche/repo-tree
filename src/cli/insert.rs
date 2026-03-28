//! Insert a repository existing outside the repo tree, within it.
use std::fs::create_dir_all;
use std::fs::remove_dir;
use std::fs::rename;
use std::path::PathBuf;

use clap::Args;
use clap_complete::ArgValueCompleter;
use clap_complete::PathCompleter;

use crate::config::Config;
use crate::repository::Repositories;
use crate::repository::Repository;

/// Clone a repository within the repo tree.
#[derive(Args)]
pub struct InsertArgs {
    /// Path to the repository to insert.
    #[arg(add=ArgValueCompleter::new(PathCompleter::dir()))]
    path: String,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Refresh the repositories cache based on the refresh_cache boolean value.
fn refresh_cache(config: &Config, refresh_cache: bool) {
    if refresh_cache {
        Repositories::load(config, true);
    }
}

/// Execute the `rt insert` command.
pub fn run(config: &Config, args: InsertArgs) -> i32 {
    let repository =
        match Repository::discover_silent(config, PathBuf::from(args.path)) {
            Ok(r) => r,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        };

    let expected_root = match repository.expected_root(config) {
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

    if repository.root == expected_root {
        eprintln!("Repository already at the correct location");
        refresh_cache(config, args.refresh_cache);
        return 0;
    }

    let parent = expected_root.parent().unwrap();

    if !parent.exists()
        && let Err(err) = create_dir_all(parent)
    {
        eprintln!("{err}");
        refresh_cache(config, args.refresh_cache);
        return 1;
    }

    if let Err(err) = rename(&repository.root, &expected_root) {
        eprintln!("{err}");
        refresh_cache(config, args.refresh_cache);
        return 1;
    }
    println!(
        "{} moved to {}",
        repository.root.display(),
        expected_root.display()
    );

    let mut current = repository.root.as_path();
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
    refresh_cache(config, true);

    0
}
