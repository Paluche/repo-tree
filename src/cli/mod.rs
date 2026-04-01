use std::{env, fs::canonicalize, path::PathBuf, process::exit};

use clap::{Parser, Subcommand};

mod clean;
mod clone;
mod complete_env;
mod fetch;
mod git;
mod list;
mod repo;
mod resolve;
mod resolve_url;
mod rm;
mod todo;
mod tree;
mod util;

use crate::Config;

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
    ResolveUrl(resolve_url::ResolveUrlArgs),
    Clone(clone::CloneArgs),
    List(list::ListArgs),
    Tree(tree::TreeArgs),
    Clean(clean::CleanArgs),
    Fetch(fetch::FetchArgs),
    Todo(todo::TodoArgs),
    Repo(repo::RepoArgs),
    Git(git::GitArgs),
    Util(util::UtilArgs),
    Rm(rm::RmArgs),
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

/// Entry point for the executable.
pub fn run() -> i32 {
    complete_env::complete();

    let args = Args::parse();
    let config = match Config::load() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

    match args.action {
        Action::Resolve(args) => resolve::run(&config, args),
        Action::ResolveUrl(args) => resolve_url::run(&config, args),
        Action::List(args) => list::run(&config, args),
        Action::Tree(args) => tree::run(&config, args),
        Action::Clean(args) => clean::run(&config, args),
        Action::Fetch(args) => fetch::run(&config, args),
        Action::Todo(args) => todo::run(&config, args),
        Action::Repo(args) => repo::run(&config, args),
        Action::Git(args) => git::run(&config, args),
        Action::Clone(args) => clone::run(&config, args),
        Action::Util(args) => util::run(&config, args),
        Action::Rm(args) => rm::run(&config, args),
    }
}
