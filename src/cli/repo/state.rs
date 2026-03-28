//! Print the state of the repository.
use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::repository::Repositories;
use crate::repository::Repository;

/// Find out if there is something to do by the user in order to keep this
/// repository updated.
#[derive(Args)]
pub struct StateArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt repo state` command.
pub fn run(config: &Config, args: StateArgs) -> i32 {
    if args.refresh_cache {
        Repositories::load(config, true);
    }

    let repository =
        match Repository::discover(config, cwd_default_path(args.repository)) {
            Ok(r) => r,
            Err(err) => {
                eprintln!("Error: {err}");
                return 1;
            }
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
