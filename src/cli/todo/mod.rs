use clap::{Args, Subcommand};

mod list;

/// Actions for git repositories.
#[derive(Args, Debug, PartialEq)]
pub struct TodoArgs {
    #[command(subcommand)]
    action: TodoAction,
}

#[derive(Subcommand, Debug, PartialEq)]
enum TodoAction {
    List(list::ListArgs),
}

pub fn run(args: TodoArgs) -> i32 {
    match args.action {
        TodoAction::List(args) => list::run(args),
    }
}
