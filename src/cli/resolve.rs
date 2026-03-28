//! Action to resolve the path to a repository from its name or alias.

use clap::Args;
use clap_complete::engine::ArgValueCompleter;

use crate::config::Config;
use crate::repository::Repositories;
use crate::resolve::resolve;
use crate::resolve::resolve_completer;

/// Resolve the name of a repository into its path.
#[derive(Args)]
pub struct ResolveArgs {
    /// Repository identifier to resolve into the actual path within the
    /// repo_tree.
    #[arg(add=ArgValueCompleter::new(resolve_completer))]
    repo_id: Option<String>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt resolve` command.
pub fn run(config: &Config, args: ResolveArgs) -> i32 {
    let repositories = Repositories::load(config, args.refresh_cache);
    if let Some(repository) = match resolve(config, &repositories, args.repo_id)
    {
        Ok(r) => r,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    } {
        println!("{}", repository.root.display());
        0
    } else {
        2
    }
}
