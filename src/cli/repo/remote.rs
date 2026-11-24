//! Compute the root to the repository.
use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;
use colored::Colorize;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::ui::Ui;
use crate::version_control_system::VcsRemote;

/// Get the remote associated with the repository.
#[derive(Args)]
pub struct RemoteArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Select among the available remotes. The remotes vector must have more than
/// one element.
fn choose_remote(remotes: &[VcsRemote]) -> &String {
    let default = remotes
        .iter()
        .find(|r| r.name == "origin")
        .or_else(|| remotes.first())
        .expect("choose_remote is called with several remotes");
    let mut choices: Vec<&VcsRemote> =
        remotes.iter().filter(|r| r.name != default.name).collect();

    choices.insert(0, default);

    for (i, remote) in choices.iter().enumerate() {
        eprintln!(
            " {}: {} {}{}",
            i,
            remote.name.bold(),
            remote.url.bold().cyan(),
            if i == 0 { " (default)" } else { "" }
        );
    }

    // Failing to read the choice selects the default remote.
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok()
        && let Ok(choice) = input.trim().parse::<usize>()
        && choice < choices.len()
    {
        &choices[choice].url
    } else {
        &default.url
    }
}

/// Execute the `rt repo remote` command.
pub async fn run(ui: &mut Ui<'_>, config: &Config, args: RemoteArgs) -> i32 {
    if args.refresh_cache {
        RepoTree::load(ui, config, true);
    }

    let remotes = match Repository::discover(
        ui,
        config,
        &cwd_default_path(ui, args.repository),
        ExpectedTreeStrategy::Lazy,
    )
    .await
    .and_then(|r| r.get_vcs_repo(r.get_latest_workspace()).get_remotes())
    {
        Ok(r) => r,
        Err(err) => {
            ui.error(err);
            return 1;
        }
    };

    if remotes.is_empty() {
        ui.error("No remote URL found for the repository");
        1
    } else if remotes.len() == 1 {
        println!("{}", remotes[0].url);
        0
    } else {
        println!("{}", choose_remote(&remotes));
        0
    }
}
