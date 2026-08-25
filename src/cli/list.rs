//! List your repositories.
use clap::ArgAction;
use clap::Args;
use globset::Glob;

use crate::config::Config;
use crate::repo_tree::RepoTree;

/// List all repositories in the repo_tree.
#[derive(Args)]
pub struct ListArgs {
    /// Filter the repositories to list by their host. For example, "github" or
    /// "local". You can specify glob patterns. Can be specified multiple times
    /// as an union filter.
    #[arg(
        short='H', long="host", action=ArgAction::Append,
        add=Config::host_completer()
        )
    ]
    hosts: Vec<Glob>,
    /// Filter the repositories to list by their name. You can specify glob
    /// patterns. For example to filter only GitHub repositories from a
    /// certain organization (e.g. 'owner'), you could use the 'owner/*' as
    /// value for this argument, and "github" as value of the --host
    /// argument. Can be specified multiple times as an union filter.
    #[arg(short = 'N', long = "name", action=ArgAction::Append)]
    names: Vec<Glob>,
    /// Filter the repositories to list by the tree-space they belong to. You
    /// can specify glob patterns. For example to filter only archived
    /// repositories you could use the "archive" value for this argument.
    /// Can be specified multiple times as an union filter.
    #[arg(short = 'T', long = "tree", action=ArgAction::Append)]
    trees: Vec<Glob>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt list` command.
pub fn run(config: &Config, args: ListArgs) -> i32 {
    for (_, workspace) in RepoTree::load(config, args.refresh_cache)
        .filtered(config, &args.hosts, &args.names, &args.trees)
        .iter()
    {
        println!("{}", workspace.path().display());
    }
    0
}
