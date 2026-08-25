//! Print the state of the repository.
use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::repository::Repository;
use crate::tree::RepoTree;

/// Find out if there is something to do by the user in order to keep this
/// repository updated.
#[derive(Args)]
pub struct StateArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
    /// Verbose mode, print all available information on the repository
    /// alongside its state.
    #[arg(short, long)]
    verbose: bool,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt repo state` command.
pub async fn run(config: &Config, args: StateArgs) -> i32 {
    if args.refresh_cache {
        RepoTree::load(config, true);
    }

    let repository = match Repository::discover(
        config,
        &cwd_default_path(args.repository),
    ) {
        Ok(r) => r,
        Err(err) => {
            eprintln!("Error: {err}");
            return 1;
        }
    };

    if args.verbose {
        if let Some(name) = repository.id.remote_host_name(config) {
            print!("{name} ");
        }

        if let Some(repr) = repository.id.remote_host_repr(config) {
            print!("{repr} ");
        }

        println!(
            "{}{}",
            repository.id.name,
            repository
                .id
                .remote
                .as_ref()
                .map_or("".to_string(), |r| format!(": {}", r.url))
        );
        println!(
            "{} {}",
            repository.vcs,
            repository.vcs.short_display(config)
        );
    }

    let repo_state = match repository.state().await {
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
