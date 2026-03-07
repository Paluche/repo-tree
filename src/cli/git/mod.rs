use clap::{Args, Subcommand};

mod status_action;

/// Actions for git repositories.
#[derive(Args, Debug, PartialEq)]
pub struct GitArgs {
    #[command(subcommand)]
    action: GitAction,
}

#[derive(Subcommand, Debug, PartialEq)]
enum GitAction {
    Status(status_action::StatusArgs),
}

pub fn run(args: GitArgs) -> i32 {
    match args.action {
        GitAction::Status(args) => status_action::run(args),
    }
}
