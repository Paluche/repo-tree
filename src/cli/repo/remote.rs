//! Compute the root to the repository.
use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;
use colored::Colorize;

use crate::version_control_system::VcsRemote;
use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::ui::Ui;

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
        .unwrap_or(remotes.first().unwrap());
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

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    if let Ok(choice) = input.trim().parse::<usize>()
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
            println!("{err}");
            return 1;
        }
    };

    if remotes.is_empty() {
        eprintln!("No remote URL found for the repository");
        1
    } else if remotes.len() == 1 {
        println!("{}", remotes[0].url);
        0
    } else {
        println!("{}", choose_remote(&remotes));
        0
    }
}
