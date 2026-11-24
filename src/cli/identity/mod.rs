//! Sub-commands to manage the identities used in the repositories.
mod apply;
mod setup_git;
mod show;

use clap::Args;
use clap::Subcommand;

use crate::config::Config;
use crate::ui::Ui;

/// Manage the identities used in the repositories.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Args)]
pub struct IdentityArgs {
    #[command(subcommand)]
    action: IdentityAction,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Subcommand)]
enum IdentityAction {
    Show(show::ShowArgs),
    Apply(apply::ApplyArgs),
    SetupGit(setup_git::SetupGitArgs),
}

/// Execute the `rt identity` sub-commands.
pub async fn run(ui: &mut Ui<'_>, config: &Config, args: IdentityArgs) -> i32 {
    match args.action {
        IdentityAction::Show(args) => show::run(ui, config, args).await,
        IdentityAction::Apply(args) => apply::run(ui, config, args).await,
        IdentityAction::SetupGit(args) => setup_git::run(ui, config, &args),
    }
}
