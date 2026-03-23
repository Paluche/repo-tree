use clap::{ArgAction, Args};
use clap_complete::engine::ArgValueCompleter;
use colored::Colorize;
use crossterm::terminal::{Clear, ClearType};

use crate::{
    NotImplementedError, Repository,
    cli::cwd_default_path,
    config::{Config, list_host_completer},
    load_filtered_repositories,
};

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

pub fn run(args: NextPrevArgs, reverse: bool) -> i32 {
    let repo_path = cwd_default_path(None);
    let config = Config::default();
    let current_repository = Repository::discover(&config, repo_path.clone())
        .expect("Error loading the repository");

    let repositories =
        load_filtered_repositories(&config, args.hosts, args.names);

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
                repository.id.host.map_or("".red().to_string(), |h| h.repr),
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
