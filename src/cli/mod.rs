use std::{env, fs::canonicalize, io, path::PathBuf, process::exit};

use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{
    CompleteEnv, Generator, PathCompleter, Shell, engine::ArgValueCompleter,
    generate,
};

mod clean;
mod fetch;
mod git;
mod list;
mod prompt;
mod repo;
mod resolve;
mod tree;

use clean::clean;
use fetch::fetch;
use git::{GitAction, run_git};
use list::{list, list_host_completer};
pub use prompt::PromptBuilder;
use prompt::prompt;
use repo::{RepoAction, run_repo};
use resolve::{resolve, resolve_completer};
use tree::tree;

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
        /// repo_tree.
        #[arg(add=ArgValueCompleter::new(resolve_completer))]
        repo_id: Option<String>,
    },
    /// List all repositories in the repo_tree.
    List {
        /// Filter the repositories to list by their host. For example,
        /// "github" or "local".
        #[arg(short='H', long, add=ArgValueCompleter::new(list_host_completer))]
        host: Option<String>,
        /// Filter the repositories to by their name, within its forge. All
        /// repositories which name starts with the provided value will be
        /// listed. For example to filter only GitHub repositories from a
        /// certain organization, you could use the organization name as value
        /// for this argument, and "github" as value of the --host argument.
        #[arg(short = 'N', long)]
        name: Option<String>,
    },
    /// Display a tree of your repo_tree.
    Tree,
    /// Clean the repo_tree. Move the repositories where they belong and remove
    /// empty directories.
    Clean {
        /// Do not perform any change on the repo_tree. Simply print what would
        /// be done.
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Fetch all the repositories within the repo_tree.
    Fetch {
        /// Suppress output to the minimum, only the final summary will be
        /// printed.
        #[arg(short, long)]
        quiet: bool,
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
        Action::List { host, name } => list(host, name),
        Action::Tree => tree(),
        Action::Clean { dry_run } => clean(dry_run),
        Action::Fetch { quiet } => fetch(quiet),
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
