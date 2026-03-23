//! Compute the root to the repository.
use clap::Args;
use clap_complete::{PathCompleter, engine::ArgValueCompleter};

use crate::{Config, Repository, cli::cwd_default_path};

/// Get the root and type of the repository the working directory or its
/// parent is into.
#[derive(Args, Debug, PartialEq)]
pub struct RemoteArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
}

pub fn run(args: RemoteArgs) -> i32 {
    let repo_path = cwd_default_path(args.repository);
    if let Some(repository) =
        Repository::discover(&Config::default(), repo_path.clone())
            .expect("Error loading the repository")
    {
        if let Some(remote_url) = repository.id.remote_url {
            println!("{remote_url}");
            0
        } else {
            eprintln!("No remote URL found for the repository");
            1
        }
    } else {
        eprintln!("Not within a repository");
        1
    }
}
