use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{
    CompleteEnv, Generator, PathCompleter, Shell, engine::ArgValueCompleter,
    generate,
};
use std::{env, fs::canonicalize, io, path::PathBuf, process::exit};

mod clean_action;
mod git;
mod prompt_action;
mod resolve_action;
mod tree_action;

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
        #[arg(add=ArgValueCompleter::new(resolve_action::resolve_completer))]
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
    /// Actions for git repositories.
    Git {
        #[command(subcommand)]
        action: git::GitAction,
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

fn cwd_default_path(path: Option<String>) -> PathBuf {
    let ret = path.map_or_else(|| env::current_dir().unwrap(), PathBuf::from);

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
        Action::Resolve { repo_id } => resolve_action::resolve(repo_id),
        Action::Tree => tree_action::tree(),
        Action::Clean { dry_run } => clean_action::clean(dry_run),
        Action::Git { action } => git::run_git(action),
        Action::Prompt { repository } => {
            prompt_action::prompt(cwd_default_path(repository))
        }
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
