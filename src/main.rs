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
use git2::Repository;
use std::{env, io, path::Path, process::exit};

use repo_prompt::url_parsing::parse_repo_url;

#[derive(Parser, Debug, PartialEq)]
#[command(version, about, long_about = None)]
struct Args {
    /// Action to perform
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Action {
    Prompt {
        /// Path to within the git repository to work with.
        #[arg(short, long)]
        repository: Option<String>,
    },
    Status {
        /// Path to within the git repository to work with.
        #[arg(short, long)]
        repository: Option<String>,
    },
    Resolve {
        /// Path to within the git repository to work with.
        #[arg(short, long)]
        repository: Option<String>,
    },
    Completion {
        shell: Shell,
    },
}

fn main() {
    let args = Args::parse();

    match args.action {
        Action::Completion { shell } => {
            generate_completion(&mut Args::command(), shell);
        }
        Action::Prompt { repository } => prompt(repository),
        Action::Status { .. } => panic!("Not Implemented yet"),
        Action::Resolve { .. } => panic!("Not Implemented yet"),
    }
}

fn prompt(repo_path: Option<String>) {
    let repo = load_repository(repo_path);
    let (forge, repo_path) = parse_repo_url(&repo).unwrap();

    let work_dir = env::var("WORK_DIR").unwrap();
    let mut expected_path = Path::new(&work_dir).to_path_buf();
    expected_path.push(forge);
    expected_path.push(&repo_path);
    let expected_path = expected_path.as_path();
    let current_repo_path = repo.workdir().unwrap();

    if current_repo_path == expected_path {
        eprintln!(
            "⚠️Unexpected location for the repository {}. Currently in \"{}\" \
            should be in \"{}\".",
            repo_path,
            current_repo_path.display(),
            expected_path.display(),
        );
    }

    // prompt is |type|repo_path|branch/bookmark[|ongoing git operation]|status|[submodule_status]
    // type is git / jj / svn (emojis?)
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
