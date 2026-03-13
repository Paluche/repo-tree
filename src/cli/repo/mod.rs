mod remote;
mod root;

use clap::{Args, Subcommand};

/// Actions for any type of repository.
#[derive(Args, Debug, PartialEq)]
pub struct RepoArgs {
    #[command(subcommand)]
    action: RepoAction,
}

#[derive(Subcommand, Debug, PartialEq)]
enum RepoAction {
    Root(root::RootArgs),
    Remote(remote::RemoteArgs),
}

pub fn run(args: RepoArgs) -> i32 {
    match args.action {
        RepoAction::Root(args) => root::run(args),
        RepoAction::Remote(args) => remote::run(args),
    }
}
