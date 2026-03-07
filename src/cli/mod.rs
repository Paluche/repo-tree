use std::{env, fs::canonicalize, path::PathBuf, process::exit};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;

mod clean;
mod completion;
mod fetch;
mod git;
mod list;
mod prompt;
mod repo;
mod resolve;
mod tree;

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
    Resolve(resolve::ResolveArgs),
    List(list::ListArgs),
    Tree(tree::TreeArgs),
    Clean(clean::CleanArgs),
    Fetch(fetch::FetchArgs),
    Repo(repo::RepoArgs),
    Git(git::GitArgs),
    Prompt(prompt::PromptArgs),
    Completion(completion::CompletionArgs),
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
        canonicalize(env::current_dir().unwrap().join(ret)).unwrap()
    } else {
        ret
    }
}

pub fn run() -> i32 {
    CompleteEnv::with_factory(Args::command).complete();

    let args = Args::parse();

    match args.action {
        Action::Resolve(args) => resolve::run(args),
        Action::List(args) => list::run(args),
        Action::Tree(args) => tree::run(args),
        Action::Clean(args) => clean::run(args),
        Action::Fetch(args) => fetch::run(args),
        Action::Repo(args) => repo::run(args),
        Action::Git(args) => git::run(args),
        Action::Prompt(args) => prompt::run(args),
        Action::Completion(args) => completion::run(&mut Args::command(), args),
    }
}
