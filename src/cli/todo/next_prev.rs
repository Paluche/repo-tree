//! Obtain the path to the next or previous repository where there is something
//! to be done by the user.
use clap::ArgAction;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::config::list_host_completer;
use crate::error::NoRepositoryError;
use crate::error::NotImplementedError;
use crate::repository::Repositories;
use crate::repository::Repository;
use crate::utils::into_iter_from;

/// Go to the next or previous repository where you have to do something to keep
/// it up-to-date.
#[derive(Args)]
pub struct NextPrevArgs {
    /// Filter the repositories to list by their host. For example, "github" or
    /// "local". Can be specified multiple times.
    #[arg(
        short='H', long="host", action=ArgAction::Append,
        add=ArgValueCompleter::new(list_host_completer)
        )
    ]
    hosts: Vec<String>,
    /// Filter the repositories to by their name, within its forge. All
    /// repositories which name starts with the provided value will be
    /// listed. For example to filter only GitHub repositories from a
    /// certain organization, you could use the organization name as value
    /// for this argument, and "github" as value of the --host argument. Can be
    /// specified multiple times.
    #[arg(short = 'N', long = "name", action=ArgAction::Append)]
    names: Vec<String>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt todo next` or `rt todo prev` command.
pub fn run(config: &Config, args: NextPrevArgs, reverse: bool) -> i32 {
    let repo_path = cwd_default_path(None);
    let current_repository =
        match Repository::discover(config, repo_path.clone()) {
            Ok(r) => Some(r),
            Err(err) => {
                if err.downcast_ref::<NoRepositoryError>().is_none() {
                    eprintln!("Error: {err}");
                    return 1;
                }
                None
            }
        };

    let repositories = Repositories::load(config, args.refresh_cache);

    // Skip the current repository.
    for repository in into_iter_from(
        repositories.filtered(config, args.hosts, args.names),
        &current_repository,
        reverse,
    ) {
        if repository.id.remote.is_local() {
            continue;
        }
        eprint!("\r{}{}", Clear(ClearType::CurrentLine), repository.id.name);
        if let Some(repo_state) = match &repository.state() {
            Ok(v) => Some(v),
            Err(err) => {
                if err.downcast_ref::<NotImplementedError>().is_some() {
                    None
                } else {
                    eprintln!("{err}");

                    return 1;
                }
            }
        } {
            if repo_state.is_ok() {
                continue;
            }
            eprintln!(
                "\r{}{} {:20} {}",
                Clear(ClearType::CurrentLine),
                repository.id.remote.host(config).repr(),
                repository.id.name,
                repo_state
            );
            println!("{}", repository.root.display());
            return 0;
        }
    }

    eprint!("\r{}", Clear(ClearType::CurrentLine));
    eprintln!("Nothing to do.");
    0
}
