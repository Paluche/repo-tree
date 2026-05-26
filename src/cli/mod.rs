//! Definition of the rt CLI.
use std::env;
use std::fs::canonicalize;
use std::path::PathBuf;
use std::process::exit;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

mod clean;
mod clone;
mod complete_env;
mod fetch;
mod git;
mod insert;
mod list;
mod refresh_cache;
mod repo;
mod resolve;
mod resolve_url;
mod rm;
mod test;
mod todo;
mod tree;

use crate::config::Config;
use crate::repo_id::ExpectedTreeStrategy;

/// Control of the colored output.
#[derive(Default, Clone, ValueEnum)]
pub enum ColorBehavior {
    /// Automatically enable or disable colors based on the type of output
    /// (default).
    #[default]
    Auto,
    /// Never color the colored output.
    Never,
    /// Always color the colored output.
    Always,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Action to perform.
    #[command(subcommand)]
    action: Action,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
    /// Force output color behavior.
    #[arg(long, global = true, value_enum, default_value_t)]
    color: ColorBehavior,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Subcommand)]
enum Action {
    Resolve(resolve::ResolveArgs),
    ResolveUrl(resolve_url::ResolveUrlArgs),
    Clone(clone::CloneArgs),
    Insert(insert::InsertArgs),
    List(list::ListArgs),
    Tree(tree::TreeArgs),
    Clean(clean::CleanArgs),
    Fetch(fetch::FetchArgs),
    Test(test::TestArgs),
    Todo(todo::TodoArgs),
    Repo(repo::RepoArgs),
    Git(git::GitArgs),
    Rm(rm::RmArgs),
    RefreshCache(refresh_cache::RefreshCacheArgs),
}

/// Get the path to the current working directory.
fn get_cwd() -> PathBuf {
    env::current_dir()
        .inspect_err(|_| {
            eprintln!("Current directory does not exist");
            exit(1);
        })
        .unwrap()
}

/// Process path arguments, which should default to the current working
/// directory if not specified.
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

/// Possible values to force the tree-space one repository can be added in to.
#[derive(Clone, Debug, ValueEnum)]
pub enum ForceTreeSpace {
    /// Force the repository to be added in the dev tree-space, if the
    /// repository has a remote configured.
    Dev,
    /// Force the repository to be added in the archive tree-space, if the
    /// repository has a remote configured.
    Archive,
}

/// Convert the force_tree argument into a ExpectedTreeStrategy.
pub fn force_tree_into_strategy(
    force_tree: Option<ForceTreeSpace>,
) -> ExpectedTreeStrategy {
    match force_tree {
        None => ExpectedTreeStrategy::Exact,
        Some(ForceTreeSpace::Dev) => ExpectedTreeStrategy::ForceDev,
        Some(ForceTreeSpace::Archive) => ExpectedTreeStrategy::ForceArchive,
    }
}

/// Entry point for the executable.
pub async fn run() -> i32 {
    complete_env::complete();

    let args = Args::parse();
    let config = match Config::load() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

    match args.color {
        ColorBehavior::Auto => (),
        ColorBehavior::Always => colored::control::set_override(true),
        ColorBehavior::Never => colored::control::set_override(false),
    }

    match args.action {
        Action::Resolve(args) => resolve::run(&config, args),
        Action::ResolveUrl(args) => resolve_url::run(&config, args),
        Action::List(args) => list::run(&config, args),
        Action::Tree(args) => tree::run(&config, args),
        Action::Clean(args) => clean::run(&config, args).await,
        Action::Fetch(args) => fetch::run(&config, args),
        Action::Todo(args) => todo::run(&config, args).await,
        Action::Repo(args) => repo::run(&config, args).await,
        Action::Git(args) => git::run(&config, args).await,
        Action::Clone(args) => clone::run(&config, args).await,
        Action::Rm(args) => rm::run(&config, args).await,
        Action::RefreshCache(args) => refresh_cache::run(&config, args),
        Action::Insert(args) => insert::run(&config, args).await,
        Action::Test(args) => test::run(&config, args).await,
    }
}
