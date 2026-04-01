//! Compute the root to the repository.
use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;

use crate::Config;
use crate::Repository;
use crate::cli::cwd_default_path;

/// Get the root and type of the repository the working directory or its
/// parent is into.
#[derive(Args, Debug, PartialEq)]
pub struct RemoteArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
}

/// Execute the `rt repo remote` command.
pub fn run(config: &Config, args: RemoteArgs) -> i32 {
    let repo_path = cwd_default_path(args.repository);
    if let Some(repository) = Repository::discover(config, repo_path.clone())
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
