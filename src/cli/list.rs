use clap::{ArgAction, Args};
use clap_complete::engine::ArgValueCompleter;

use crate::{
    UrlParser,
    config::{Config, list_host_completer},
    get_repo_tree_dir, load_repo_tree,
};

/// List all repositories in the repo_tree.
#[derive(Args, Debug, PartialEq)]
pub struct ListArgs {
    /// Filter the repositories to list by their host. For example, "github" or
    /// "local".
    #[arg(short='H', long, add=ArgValueCompleter::new(list_host_completer))]
    host: Option<String>,
    /// Filter the repositories to by their name, within its forge. All
    /// repositories which name starts with the provided value will be
    /// listed. For example to filter only GitHub repositories from a
    /// certain organization, you could use the organization name as value
    /// for this argument, and "github" as value of the --host argument.
    #[arg(short = 'N', long = "name", action=ArgAction::Append)]
    names: Vec<String>,
}

pub fn run(args: ListArgs) -> i32 {
    let repositories = load_repo_tree(
        &get_repo_tree_dir(),
        &UrlParser::new(&Config::default()),
    )
    .0;

    for repository in repositories {
        if let Some(host) = &args.host {
            if let Some(repo_host) = repository.id.host {
                if &repo_host.name != host {
                    continue;
                }
            } else if host != "local" {
                continue;
            }
        }

        if !args.names.is_empty()
            && !args
                .names
                .iter()
                .any(|name| repository.id.name.starts_with(name))
        {
            continue;
        }

        println!("{}", repository.root.display());
    }
    0
}
