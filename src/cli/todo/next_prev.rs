//! Obtain the path to the next or previous repository where there is something
//! to be done by the user.
use clap::ArgAction;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;
use colored::Colorize;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;

use crate::NotImplementedError;
use crate::Repository;
use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::config::list_host_completer;
use crate::error::NoRepositoryError;
use crate::load_filtered_repositories;

/// Go to the next or previous repository where you have to do something to keep
/// it up-to-date.
#[derive(Args, Debug, PartialEq)]
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
}

/// Iterate the repositories, starting from the specified one.
fn iter_repos_from(
    repositories: Vec<Repository>,
    start: Option<Repository>,
) -> Box<dyn DoubleEndedIterator<Item = Repository>> {
    if let Some(start) = start {
        // Use partition_in_place when stable.
        let mut start_found = false;
        let (start, end): (Vec<Repository>, Vec<Repository>) =
            repositories.into_iter().partition(move |r| {
                if r == &start {
                    start_found = true;
                }
                start_found
            });

        Box::new(start.into_iter().skip(1).chain(end))
    } else {
        Box::new(repositories.into_iter())
    }
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

    let repositories =
        load_filtered_repositories(config, args.hosts, args.names);

    let mut repositories = iter_repos_from(repositories, current_repository);

    if reverse {
        repositories = Box::new(repositories.rev());
    }

    // Skip the current repository.
    for repository in repositories {
        if repository.id.remote_url.is_none() {
            // Local repository.
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
                repository
                    .id
                    .host
                    .map_or("".red().to_string(), |h| h.repr()),
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
