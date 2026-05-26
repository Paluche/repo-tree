//! Definition of the rt CLI.
use std::env;
use std::error::Error;
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
mod workspace;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::repository::Workspace;
use crate::resolve::ResolveFilter;
use crate::resolve::resolve;
use crate::tree_space::TreeSpaceKind;
use crate::ui::Ui;

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
    Workspace(workspace::WorkspaceArgs),
    Git(git::GitArgs),
    Rm(rm::RmArgs),
    RefreshCache(refresh_cache::RefreshCacheArgs),
}

/// Get the path to the current working directory.
fn get_cwd(ui: &Ui<'_>) -> PathBuf {
    env::current_dir()
        .inspect_err(|_| {
            ui.error("Current directory does not exist");
            exit(1);
        })
        .unwrap()
}

/// Process path arguments, which should default to the current working
/// directory if not specified.
fn cwd_default_path(ui: &Ui, path: Option<String>) -> PathBuf {
    let ret = path.map_or_else(|| get_cwd(ui), PathBuf::from);

    if !ret.exists() {
        ui.error(format!("No such directory {}", ret.display()));
        exit(1);
    }

    if !ret.is_absolute() {
        canonicalize(env::current_dir().unwrap().join(ret)).unwrap()
    } else {
        ret
    }
}

/// Get the repository in the current working directory or, if a repository ID is provided get the
/// asked repository.
fn get_current_repo_or_main<'repo_tree>(
    config: &Config,
    ui: &Ui<'_>,
    repo_tree: &'repo_tree RepoTree,
    repo_id: Option<String>,
) -> Result<
    Option<(&'repo_tree Repository, &'repo_tree Workspace)>,
    Box<dyn Error>,
> {
    if let Some(repo_id) = repo_id {
        resolve(
            config,
            ui,
            repo_tree,
            Some(repo_id),
            ResolveFilter::TreeSpaceKind(TreeSpaceKind::Main),
        )
    } else {
        Ok(repo_tree.get_workspace(&get_cwd(ui)))
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
    let mut ui = Ui::new(&config);

    match args.color {
        ColorBehavior::Auto => (),
        ColorBehavior::Always => colored::control::set_override(true),
        ColorBehavior::Never => colored::control::set_override(false),
    }

    match args.action {
        Action::Resolve(args) => resolve::run(&config, &ui, args),
        Action::ResolveUrl(args) => resolve_url::run(&config, &ui, args),
        Action::List(args) => list::run(&config, &ui, args),
        Action::Tree(args) => tree::run(&config, &ui, args),
        Action::Clean(args) => clean::run(&config, &mut ui, args).await,
        Action::Fetch(args) => fetch::run(&config, &ui, args),
        Action::Todo(args) => todo::run(&config, &mut ui, args).await,
        Action::Repo(args) => repo::run(&config, &mut ui, args).await,
        Action::Workspace(args) => workspace::run(&config, &ui, args).await,
        Action::Git(args) => git::run(&config, &mut ui, args).await,
        Action::Clone(args) => clone::run(&config, &mut ui, args).await,
        Action::Rm(args) => rm::run(&config, &ui, args).await,
        Action::RefreshCache(args) => refresh_cache::run(&config, &ui, args),
        Action::Insert(args) => insert::run(&config, &mut ui, args).await,
        Action::Test(args) => test::run(&config, args).await,
    }
}
