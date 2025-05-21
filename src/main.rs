//! Goal: Reproduce my git prompt done in shell + python.
//! The displayed information are:
//! - Repo name: Either the origin URL path.
//! - Which reference we are on
//! - ongoing operation (if there is one)
//! - ahead behind
//! - schematic git status
//! - schematic submodule status
//!
//! Custom Git status:
//! + Add remotes list
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use git2::{Repository, Remote};
use std::{env, io, process::exit};

#[derive(Parser, Debug, PartialEq)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to within the git repository to work with.
    #[arg(short, long)]
    repository: Option<String>,
    /// Action to perform
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Action {
    Prompt,
    Status,
    Completion { shell: Shell },
}

fn main() {
    let args = Args::parse();

    match args.action {
        Action::Completion { shell } => {
            generate_completion(&mut Args::command(), shell);
        }
        Action::Prompt => prompt(args.repository),
        Action::Status => panic!("Not Implemented yet"),
    }
}

fn prompt(repo_path: Option<String>) {
    let repo = load_repository(repo_path);

    load_repo_name(&repo);
}

fn load_default_remote(repo: &Repository) -> Option<Remote> {
    let remotes = repo.remotes().unwrap();

    if remotes.is_empty() {
        None
    } else {
        Some(
            match repo.find_remote("origin") {
                Ok(remote) => remote,
                Err(_) => repo.find_remote( remotes.get(0)?).unwrap()
            }
        )
    }
}

fn load_repo_name(repo: &Repository) -> Option<String> {
    let default_remote = load_default_remote(repo)?;
    let url = default_remote.url().unwrap();
    println!("{} {}", default_remote.name().unwrap(), url);

    Some(url.to_string())
}

fn generate_completion<G: Generator + std::fmt::Debug>(
    command: &mut Command,
    generator: G,
) {
    eprintln!("Generating completion file for {generator:?}...");
    generate(
        generator,
        command,
        command.get_name().to_string(),
        &mut io::stdout(),
    );
}

fn load_repository(repo_path: Option<String>) -> Repository {
    let repo_path = repo_path
        .unwrap_or(String::from(env::current_dir().unwrap().to_str().unwrap()));
    Repository::discover(repo_path)
        .inspect_err(|e| {
            println!("{}", e.message());
            exit(1);
        })
        .unwrap()
}
