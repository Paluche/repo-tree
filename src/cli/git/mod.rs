//! Sub-commands dedicated for Git repositories.
use clap::Args;
use clap::Subcommand;

use crate::config::Config;

mod status;

/// Actions for git repositories.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Args, Debug, PartialEq)]
pub struct GitArgs {
    #[command(subcommand)]
    action: GitAction,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
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
