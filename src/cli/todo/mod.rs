use clap::{Args, Subcommand};

mod list;
mod next_prev;

use crate::Config;

/// Actions for git repositories.
#[derive(Args, Debug, PartialEq)]
pub struct TodoArgs {
    #[command(subcommand)]
    action: TodoAction,
}

#[derive(Subcommand, Debug, PartialEq)]
enum TodoAction {
    List(list::ListArgs),
    Next(next_prev::NextPrevArgs),
    Prev(next_prev::NextPrevArgs),
}

pub fn run(config: &Config, args: TodoArgs) -> i32 {
    match args.action {
        TodoAction::List(args) => list::run(config, args),
        TodoAction::Next(args) => next_prev::run(config, args, false),
        TodoAction::Prev(args) => next_prev::run(config, args, true),
    }
}
