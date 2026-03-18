use clap::{ArgAction, Args};
use clap_complete::engine::ArgValueCompleter;
use colored::Colorize;
use crossterm::terminal::{Clear, ClearType};
use pollster::FutureExt;

use crate::{
    UrlParser, VersionControlSystem,
    config::{Config, list_host_completer},
    get_repo_tree_dir, jujutsu, load_filtered_repositories,
};

/// Custom git status. Concise, with all the data and without help text.
#[derive(Args, Debug, PartialEq)]
pub struct ListArgs {
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
    /// Show a state for all repositories.
    #[arg(short, long, action=ArgAction::SetTrue)]
    verbose: bool,
}

pub fn run(args: ListArgs) -> i32 {
    let mut total: usize = 0;
    let mut ok: usize = 0;
    let mut n_a: usize = 0;

    for repository in load_filtered_repositories(
        &get_repo_tree_dir(),
        &UrlParser::new(&Config::default()),
        args.hosts,
        args.names,
    ) {
        if repository.id.remote_url.is_none() {
            // Local repository.
            continue;
        }
        let id = format!(
            "{} {:20}",
            repository.id.host.map_or("".red().to_string(), |h| h.repr),
            repository.id.name
        );
        eprint!("\r{}{}", Clear(ClearType::CurrentLine), repository.id.name);
        if let Some(repo_state) = match repository.vcs {
            VersionControlSystem::Jujutsu
            | VersionControlSystem::JujutsuGit => Some(
                jujutsu::get_repo_state(&repository.root)
                    .block_on()
                    .expect("Unable to obtain repository state"),
            ),
            _ => None,
        } {
            total += 1;
            if repo_state.is_ok() {
                ok += 1;
                if args.verbose {
                    eprint!("\r{}", Clear(ClearType::CurrentLine));
                    println!("{} {}", id, repo_state);
                }
            } else {
                eprint!("\r{}", Clear(ClearType::CurrentLine));
                println!("{} {}", id, repo_state);
            }
        } else {
            n_a += 1;
            if args.verbose {
                eprint!("\r{}", Clear(ClearType::CurrentLine));
                println!("{} {}", id, "N/A".bright_yellow());
            }
        }
    }

    eprint!("\r{}", Clear(ClearType::CurrentLine));
    println!("Todo: {ok}/{total} ({n_a} N/A)");
    0
}
