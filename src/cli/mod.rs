use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{
    CompleteEnv, Generator, PathCompleter, Shell, engine::ArgValueCompleter,
    generate,
};
use std::{env, fs::canonicalize, io, path::PathBuf, process::exit};

mod clean;
mod git;
mod prompt;
mod repo;
mod resolve;
mod tree;

use clean::clean;
use git::{GitAction, run_git};
use prompt::prompt;
use repo::{RepoAction, run_repo};
use resolve::{resolve, resolve_completer};
use tree::tree;

pub use prompt::PromptBuilder;

#[derive(Parser, Debug, PartialEq)]
#[command(version, about, long_about = None)]
struct Args {
    /// Action to perform
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Action {
    /// Resolve the name of a repository into its path.
    Resolve {
        /// Repository identifier to resolve into the actual path within the
        /// workspace.
        #[arg(add=ArgValueCompleter::new(resolve_completer))]
        repo_id: Option<String>,
    },
    /// Display a tree of your workspace.
    Tree,
    /// Clean the workspace. Move the repositories where they belong and remove
    /// empty directories.
    Clean {
        /// Do not perform any change on the workspace. Simply print what would
        /// be done.
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Actions for any type of repository.
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },
    /// Actions for git repositories.
    Git {
        #[command(subcommand)]
        action: GitAction,
    },
    /// Generate the prompt for your shell.
    Prompt {
        /// Path to within the repository to work with.
        #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
        repository: Option<String>,
    },
    /// Generate static completion file.
    Completion { shell: Shell },
}

fn get_cwd() -> PathBuf {
    env::current_dir()
        .inspect_err(|_| {
            eprintln!("Current directory does not exist");
            exit(1);
        })
        .unwrap()
}

fn cwd_default_path(path: Option<String>) -> PathBuf {
    let ret = path.map_or_else(get_cwd, PathBuf::from);

    if !ret.exists() {
        eprintln!("No such directory {}", ret.display());
        exit(1);
    }

    if !ret.is_absolute() {
        let mut abs = env::current_dir().unwrap();
        abs.push(ret);
        canonicalize(abs).unwrap()
    } else {
        ret
    }
}

pub fn run() -> i32 {
    CompleteEnv::with_factory(Args::command).complete();

    let args = Args::parse();

    match args.action {
        Action::Resolve { repo_id } => resolve(repo_id),
        Action::Tree => tree(),
        Action::Clean { dry_run } => clean(dry_run),
        Action::Repo { action } => run_repo(action),
        Action::Git { action } => run_git(action),
        Action::Prompt { repository } => prompt(cwd_default_path(repository)),
        Action::Completion { shell } => {
            generate_completion(&mut Args::command(), shell)
        }
    }
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
