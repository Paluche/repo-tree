//! Print the state of the repository.
use clap::Args;
use clap_complete::{PathCompleter, engine::ArgValueCompleter};

use crate::{Config, Repository, cli::cwd_default_path};

/// Find out if there is something to do by the user in order to keep this
/// repository updated.
#[derive(Args, Debug, PartialEq)]
pub struct StateArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
}

pub fn run(config: &Config, args: StateArgs) -> i32 {
    let Some(repository) =
        Repository::discover(config, cwd_default_path(args.repository))
            .expect("Error loading the repository")
    else {
        eprintln!("Not within a repository");
        return 1;
    };

    let repo_state = match repository.state() {
        Ok(v) => Some(v),
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    }
    .unwrap();

    println!("{repo_state}");
    0
}
