//! List your repositories.
use clap::ArgAction;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;
use globset::Glob;

use crate::config::Config;
use crate::config::list_host_completer;
use crate::repository::Repositories;

/// List all repositories in the repo_tree.
#[derive(Args)]
pub struct ListArgs {
    /// Filter the repositories to list by their host. For example, "github" or
    /// "local". You can specify glob patterns. Can be specified multiple times
    /// as an union filter.
    #[arg(
        short='H', long="host", action=ArgAction::Append,
        add=ArgValueCompleter::new(list_host_completer)
        )
    ]
    hosts: Vec<Glob>,
    /// Filter the repositories to by their name. You can specify glob
    /// patterns. For example to filter only GitHub repositories from a
    /// certain organization (e.g. 'owner'), you could use the 'owner/*' as
    /// value for this argument, and "github" as value of the --host
    /// argument. Can be specified multiple times as an union filter.
    #[arg(short = 'N', long = "name", action=ArgAction::Append)]
    names: Vec<Glob>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt list` command.
pub fn run(config: &Config, args: ListArgs) -> i32 {
    for repository in Repositories::load(config, args.refresh_cache)
        .filtered(config, &args.hosts, &args.names)
        .iter()
    {
        println!("{}", repository.root.display());
    }
    0
}
