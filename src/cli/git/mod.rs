//! Sub-commands dedicated for Git repositories.
use clap::{Args, Subcommand};

use crate::Config;

mod status;

/// Actions for git repositories.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Args, Debug, PartialEq)]
pub struct GitArgs {
    #[command(subcommand)]
    action: GitAction,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Subcommand, Debug, PartialEq)]
enum GitAction {
    Status(status::StatusArgs),
}

/// Execute `rt git` sub-commands.
pub fn run(config: &Config, args: GitArgs) -> i32 {
    match args.action {
        GitAction::Status(args) => status::run(config, args),
    }
}
