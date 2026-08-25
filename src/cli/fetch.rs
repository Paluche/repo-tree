//! Fetch and update the whole repo_tree.

use std::error::Error;

use clap::Args;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;

/// Fetch all the repositories within the repo_tree.
#[derive(Args)]
pub struct FetchArgs {
    /// Suppress output to the minimum, only the final summary will be printed.
    #[arg(short, long)]
    quiet: bool,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Fetch one repository.
pub fn fetch_repo(
    config: &Config,
    quiet: bool,
    repository: &Repository,
    is_submodule: bool,
) -> Result<(usize, usize), Box<dyn Error>> {
    let mut ok: usize = 0;
    let mut total: usize = 0;

    if repository.id.is_local() {
        eprintln!(
            "Skipping local repository {}",
            repository.id.display(config)
        );
        return Ok((0, 0));
    }
    if !quiet {
        println!("Fetching repository {}", repository.id.display(config));
    }
    let workspace = repository.get_main_workspace();
    for submodule in repository.submodules(workspace)? {
        let root = submodule.abs_path();
        let repository = Repository::try_new(config, &root)?;

        let (_ok, _total) = fetch_repo(config, quiet, &repository, true)?;
        ok += _ok;
        total += _total;
    }

    if !quiet {
        println!(
            "Fetching {} {}repository {}",
            repository.vcs,
            if is_submodule { "submodule " } else { "" },
            repository.id.display(config)
        );
    }

    ok += if repository.get_vcs_repo(workspace).fetch(quiet) == 0 {
        1
    } else {
        0
    };
    total += 1;

    Ok((ok, total))
}

/// Execute `rt fetch` command.
pub fn run(config: &Config, args: FetchArgs) -> i32 {
    let repo_tree = RepoTree::load(config, args.refresh_cache);

    let (ok, total) = repo_tree
        .repo_iter()
        .map(|repo| {
            fetch_repo(config, args.quiet, repo, false).unwrap_or((0, 1))
        })
        .reduce(|acc, res| (acc.0 + res.0, acc.1 + res.1))
        .unwrap_or((0, 0));

    println!("{ok}/{total} repository fetched");

    if ok == total { 0 } else { 1 }
}
