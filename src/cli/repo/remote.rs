//! Compute the root to the repository.
use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::ui::Ui;

/// Get the root and type of the repository the working directory or its
/// parent is into.
#[derive(Args)]
pub struct RemoteArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt repo remote` command.
pub async fn run(config: &Config, ui: &mut Ui<'_>, args: RemoteArgs) -> i32 {
    if args.refresh_cache {
        RepoTree::load(config, ui, true);
    }

    let repository = match Repository::discover(
        config,
        ui,
        &cwd_default_path(ui, args.repository),
        ExpectedTreeStrategy::Lazy,
    )
    .await
    {
        Ok(r) => r,
        Err(err) => {
            println!("{err}");
            return 1;
        }
    };

    if let Some(remote) = repository.id.remote {
        println!("{}", remote.url);
        0
    } else {
        eprintln!("No remote URL found for the repository");
        1
    }
}
