//! Sub-commands for a single repository.
mod prompt;
mod remote;
mod root;
mod state;

use clap::Args;
use clap::Subcommand;

use crate::config::Config;

/// Actions for any type of repository.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Args, Debug, PartialEq)]
pub struct RepoArgs {
    #[command(subcommand)]
    action: RepoAction,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Subcommand, Debug, PartialEq)]
enum RepoAction {
    Root(root::RootArgs),
    Remote(remote::RemoteArgs),
    State(state::StateArgs),
    Prompt(prompt::PromptArgs),
}

/// Execute the `rt repo` sub-commands.
pub fn run(config: &Config, args: RepoArgs) -> i32 {
    match args.action {
        RepoAction::Root(args) => root::run(config, args),
        RepoAction::Remote(args) => remote::run(config, args),
        RepoAction::State(args) => state::run(config, args),
        RepoAction::Prompt(args) => prompt::run(config, args),
    }
}
