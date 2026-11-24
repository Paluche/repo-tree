//! Set the identity in the configuration of repositories.
use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::ui::Ui;
use crate::ui_error;

/// Set the identity to use in the configuration of a repository, or of all
/// the repositories of the repo tree.
#[derive(Args)]
pub struct ApplyArgs {
    /// Path to within the repository to work with.
    #[arg(
        short,
        long,
        conflicts_with = "all",
        add=ArgValueCompleter::new(PathCompleter::dir()),
    )]
    repository: Option<String>,
    /// Apply the identities to all the repositories of the repo tree.
    #[arg(short, long)]
    all: bool,
}

/// Apply the identity to one repository and print the changes made. Return
/// whether it succeeded.
fn apply(ui: &Ui<'_>, config: &Config, repository: &Repository) -> bool {
    match repository.apply_identity(config) {
        Ok(changes) => {
            if !changes.is_empty() {
                println!("{}", repository.id.display(config));
            }
            for change in changes {
                println!("  {change}");
            }
            true
        }
        Err(err) => {
            ui_error!(
                ui,
                "Unable to set the identity of {}: {err}",
                repository.id.display(config)
            );
            false
        }
    }
}

/// Execute the `rt identity apply` command.
pub async fn run(ui: &mut Ui<'_>, config: &Config, args: ApplyArgs) -> i32 {
    if config.identities.is_empty() {
        ui.error("No identity configured");
        return 1;
    }

    if args.all {
        let repo_tree = RepoTree::load(ui, config, false);
        let mut ret = 0;

        for repository in repo_tree.repo_iter() {
            if !apply(ui, config, repository) {
                ret = 1;
            }
        }

        return ret;
    }

    match Repository::discover(
        ui,
        config,
        &cwd_default_path(ui, args.repository),
        ExpectedTreeStrategy::Lazy,
    )
    .await
    {
        Ok(repository) => i32::from(!apply(ui, config, &repository)),
        Err(err) => {
            ui.error(err);
            1
        }
    }
}
