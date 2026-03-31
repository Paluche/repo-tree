mod remote;
mod root;
mod state;

use clap::{Args, Subcommand};

use crate::Config;

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
    State(state::StateArgs),
}

pub fn run(config: &Config, args: RepoArgs) -> i32 {
    match args.action {
        RepoAction::Root(args) => root::run(config, args),
        RepoAction::Remote(args) => remote::run(config, args),
        RepoAction::State(args) => state::run(config, args),
    }
}
