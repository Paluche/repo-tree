use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{
    CompleteEnv, Generator, Shell, engine::ArgValueCompleter, generate,
};
use std::{env, io, process::exit};
use workspace::{clean, prompt, resolve, resolve_completer, status};

#[derive(Parser, Debug, PartialEq)]
#[command(version, about, long_about = None)]
struct Args {
    /// Action to perform
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Action {
    /// Generate the prompt for your shell.
    Prompt {
        /// Path to within the repository to work with.
        #[arg(short, long)]
        repository: Option<String>,
    },
    /// Print the status of the git repository.
    Status {
        /// Path to within the git repository to work with.
        #[arg(short, long)]
        repository: Option<String>,
    },
    /// Resolve the name of a repository into its path.
    Resolve {
        /// Repository identifier to resolve into the actual path within the
        /// workspace.
        #[arg(add=ArgValueCompleter::new(resolve_completer))]
        repo_id: Option<String>,
    },
    /// Generate static completion file.
    Completion { shell: Shell },
    /// Clean the workspace. Move the repositories where they belong and remove
    /// empty directories.
    Clean {
        /// Do not perform any change on the workspace. Simply print what would
        /// be done.
        #[arg(short, long)]
        dry_run: bool,
    },
}

fn get_repo_path(repository: Option<String>) -> String {
    repository
        .unwrap_or(String::from(env::current_dir().unwrap().to_str().unwrap()))
}

fn main() {
    CompleteEnv::with_factory(Args::command).complete();

    let args = Args::parse();

    exit(match args.action {
        Action::Completion { shell } => {
            generate_completion(&mut Args::command(), shell)
        }
        Action::Prompt { repository } => prompt(get_repo_path(repository)),
        Action::Status { repository } => status(get_repo_path(repository)),
        Action::Resolve { repo_id } => resolve(repo_id),
        Action::Clean { dry_run } => clean(dry_run),
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
