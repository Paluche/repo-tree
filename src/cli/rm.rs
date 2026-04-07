//! Remove a repository from the repo tree.
use std::fs::remove_dir_all;

use clap::Args;
use clap_complete::engine::ArgValueCompleter;

use crate::config::Config;
use crate::error::NotImplementedError;
use crate::repository::Repositories;
use crate::resolve::resolve;
use crate::resolve::resolve_completer;

/// Remove a repository from the repo tree.
#[derive(Args, Debug, PartialEq)]
pub struct RmArgs {
    /// Repository identifier identifying the repository to remove.
    #[arg(add=ArgValueCompleter::new(resolve_completer))]
    repo_id: Option<String>,
    /// Force the removal of the repository, even if it is not empty.
    #[arg(short, long)]
    force: bool,
}

/// Execute the `rt rm` command.
pub fn run(config: &Config, args: RmArgs) -> i32 {
    let repositories = Repositories::load(config);
    let repository = match resolve(config, &repositories, args.repo_id) {
        Ok(v) => match v {
            Some(repo) => repo,
            None => {
                eprintln!("No repository found matching the given identifier");
                return 2;
            }
        },
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

    match &repository.state() {
        Ok(repo_state) => {
            if repo_state.has_unpushed_commits() {
                eprintln!("WARNING: The repository has unpushed commits");
            }
        }
        Err(err) => {
            if err.downcast_ref::<NotImplementedError>().is_some() {
                eprintln!(
                    "Unable to check if the repository has unpushed commits"
                );
            } else {
                eprintln!("{err}");
                return 1;
            }
        }
    }

    if !args.force {
        // Ask the user for confirmation before removing the repository.
        println!(
            "Are you sure you want to remove the repository {}? [y/N]",
            repository.root.display()
        );
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() != "y" {
            println!("Aborting.");
            return 1;
        }
    }

    // Remove the repository from the repo tree.
    remove_dir_all(&repository.root).expect("Failed to remove the repository");

    // Remove parent directories if they are empty.
    let parent = &repository.root;
    while let Some(parent) = parent.parent() {
        if parent.read_dir().unwrap().next().is_none() {
            std::fs::remove_dir(parent).unwrap();
        } else {
            break;
        }
    }

    0
}
