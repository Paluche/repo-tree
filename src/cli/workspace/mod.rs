//! Sub-commands dedicated for repositories in workspace tree-spaces.
use clap::Args;
use clap::Subcommand;

use crate::config::Config;

mod add;
mod rm;

/// Actions for repositories in workspace tree-spaces.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    action: WorkspaceAction,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Subcommand)]
enum WorkspaceAction {
    Add(add::WorkspaceAddArgs),
    Rm(rm::WorkspaceRmArgs),
}

/// Execute `rt git` sub-commands.
pub async fn run(config: &Config, args: WorkspaceArgs) -> i32 {
    match args.action {
        WorkspaceAction::Add => add::run(config, args),
        WorkspaceAction::Rm => rm::run(config, args),
    }
}
