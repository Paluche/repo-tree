//! Sub-commands for a single repository.
mod prompt;
mod remote;
mod root;
mod state;

use clap::Args;
use clap::Subcommand;

use crate::config::Config;
use crate::ui::Ui;

/// Actions for any type of repository.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Args)]
pub struct RepoArgs {
    #[command(subcommand)]
    action: RepoAction,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Subcommand)]
enum RepoAction {
    Root(root::RootArgs),
    Remote(remote::RemoteArgs),
    State(state::StateArgs),
    Prompt(prompt::PromptArgs),
}

/// Execute the `rt repo` sub-commands.
pub async fn run(ui: &mut Ui<'_>, config: &Config, args: RepoArgs) -> i32 {
    match args.action {
        RepoAction::Root(args) => root::run(ui, config, args),
        RepoAction::Remote(args) => remote::run(ui, config, args).await,
        RepoAction::State(args) => state::run(ui, config, args).await,
        RepoAction::Prompt(args) => prompt::run(ui, config, args).await,
    }
}
