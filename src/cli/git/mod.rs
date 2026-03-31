use clap::{Args, Subcommand};

use crate::Config;

mod status;

/// Actions for git repositories.
#[derive(Args, Debug, PartialEq)]
pub struct GitArgs {
    #[command(subcommand)]
    action: GitAction,
}

#[derive(Subcommand, Debug, PartialEq)]
enum GitAction {
    Status(status::StatusArgs),
}

pub fn run(config: &Config, args: GitArgs) -> i32 {
    match args.action {
        GitAction::Status(args) => status::run(config, args),
    }
}
