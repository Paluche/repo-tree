//! List your repositories.
use clap::ArgAction;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;

use crate::Repositories;
use crate::config::Config;
use crate::config::list_host_completer;

/// List all repositories in the repo_tree.
#[derive(Args, Debug, PartialEq)]
pub struct ListArgs {
    /// Filter the repositories to list by their host. For example, "github" or
    /// "local". Can be specified multiple times.
    #[arg(
        short='H', long="host", action=ArgAction::Append,
        add=ArgValueCompleter::new(list_host_completer)
        )
    ]
    hosts: Vec<String>,
    /// Filter the repositories to by their name, within its forge. All
    /// repositories which name starts with the provided value will be
    /// listed. For example to filter only GitHub repositories from a
    /// certain organization, you could use the organization name as value
    /// for this argument, and "github" as value of the --host argument.
    #[arg(short = 'N', long = "name", action=ArgAction::Append)]
    names: Vec<String>,
}

/// Execute the `rt list` command.
pub fn run(config: &Config, args: ListArgs) -> i32 {
    for repository in
        Repositories::load_filtered(config, args.hosts, args.names).iter()
    {
        println!("{}", repository.root.display());
    }
    0
}
