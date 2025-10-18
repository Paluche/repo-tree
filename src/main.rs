//! repositories, prompt, custom git status,
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{Generator, Shell, generate};
use std::{env, io, process::exit};
use workspace::{prompt, resolve, status};

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
        /// Repository identifier to resolve into the actual path within the
        /// workspace.
        repo_id: String,
    },
    Completion {
        shell: Shell,
    },
}

fn get_repo_path(repository: Option<String>) -> String {
    repository
        .unwrap_or(String::from(env::current_dir().unwrap().to_str().unwrap()))
}

fn main() {
    let args = Args::parse();

    exit(match args.action {
        Action::Completion { shell } => {
            generate_completion(&mut Args::command(), shell)
        }
        Action::Prompt { repository } => prompt(get_repo_path(repository)),
        Action::Status { repository } => status(get_repo_path(repository)),
        Action::Resolve { repo_id } => resolve(repo_id),
    })
}

fn generate_completion<G: Generator + std::fmt::Debug>(
    command: &mut Command,
    generator: G,
) -> i32 {
    eprintln!("Generating completion file for {generator:?}...");
    generate(
        generator,
        command,
        command.get_name().to_string(),
        &mut io::stdout(),
    );

    0
}
