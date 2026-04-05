//! rt todo subcommands.
use clap::Args;
use clap::Subcommand;

mod list;
mod next_prev;

use crate::config::Config;

/// Commands related to the repository state. Find out if there is something to
/// do in any of the repositories of your repo tree, and help tackles then down.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Args, Debug, PartialEq)]
pub struct TodoArgs {
    #[command(subcommand)]
    action: TodoAction,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Subcommand, Debug, PartialEq)]
enum TodoAction {
    List(list::ListArgs),
    /// Go to the next repository where you have to do something to keep it
    /// up-to-date.
    Next(next_prev::NextPrevArgs),
    /// Go to the previous repository where you have to do something to keep it
    /// up-to-date.
    Prev(next_prev::NextPrevArgs),
}

/// Execute the todo subcommand.
pub fn run(config: &Config, args: TodoArgs) -> i32 {
    match args.action {
        TodoAction::List(args) => list::run(config, args),
        TodoAction::Next(args) => next_prev::run(config, args, false),
        TodoAction::Prev(args) => next_prev::run(config, args, true),
    }
}
