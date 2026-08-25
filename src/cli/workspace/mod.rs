//! Sub-commands dedicated for repositories in workspace tree-spaces.
use clap::Args;
use clap::Subcommand;

use crate::config::Config;
use crate::ui::Ui;

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
pub async fn run(ui: &Ui<'_>, config: &Config, args: WorkspaceArgs) -> i32 {
    match args.action {
        WorkspaceAction::Add(args) => add::run(ui, config, args),
        WorkspaceAction::Rm(args) => rm::run(ui, config, args),
    }
}
