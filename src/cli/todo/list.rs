//! List the operation to be done by the user in each repository.
use clap::ArgAction;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;
use colored::Colorize;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;

use crate::config::Config;
use crate::config::list_host_completer;
use crate::error::NotImplementedError;
use crate::repository::Repositories;

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

/// Execute the `rt todo list` command.
pub fn run(config: &Config, args: ListArgs) -> i32 {
    let mut todo: usize = 0;
    let mut ok: usize = 0;
    let mut n_a: usize = 0;
    let mut skipped: usize = 0;

    for repository in
        Repositories::load_filtered(config, args.hosts, args.names).iter()
    {
        let id =
            format!("{} {:20}", repository.id.host.repr(), repository.id.name);

        if repository.id.remote_url.is_none() {
            if args.verbose {
                eprint!("\r{}", Clear(ClearType::CurrentLine));
                println!("{id} {}", "Ignored (local)".bright_black());
            }
            skipped += 1;
            continue;
        }

        if config.command.todo.ignore.contains(&repository.id.name) {
            if args.verbose {
                eprint!("\r{}", Clear(ClearType::CurrentLine));
                println!("{id} {}", "Ignored (configuration)".bright_black());
            }
            skipped += 1;
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
                ok += 1;
                if args.verbose {
                    eprint!("\r{}", Clear(ClearType::CurrentLine));
                    println!("{id} {repo_state}");
                }
            } else {
                todo += 1;
                eprint!("\r{}", Clear(ClearType::CurrentLine));
                println!("{id} {repo_state}");
            }
        } else {
            n_a += 1;
            if args.verbose {
                eprint!("\r{}", Clear(ClearType::CurrentLine));
                println!("{id} {}", "N/A".bright_yellow());
            }
        }
    }

    eprint!("\r{}", Clear(ClearType::CurrentLine));
    println!("{todo} todo, {ok} OK, {n_a} N/A, {skipped} skipped");
    0
}
