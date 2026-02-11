//! Fetch and update the whole repo_tree.

use std::{error::Error, path::Path};

use clap::Args;

use crate::{
    Config, Repository, UrlParser, VersionControlSystem, get_repo_tree_dir,
    git, jujutsu, load_repo_tree,
};

/// Fetch all the repositories within the repo_tree.
#[derive(Args, Debug, PartialEq)]
pub struct FetchArgs {
    /// Suppress output to the minimum, only the final summary will be printed.
    #[arg(short, long)]
    quiet: bool,
}

pub fn fetch_repo(
    repo_tree_dir: &Path,
    url_parser: &UrlParser,
    quiet: bool,
    repository: &Repository,
    is_submodule: bool,
) -> Result<(usize, usize), Box<dyn Error>> {
    let mut ok: usize = 0;
    let mut total: usize = 0;
    if let Some(host) = &repository.id.host
        && host.name == "local"
    {
        eprintln!("Skipping local repository {}", repository.id);
        return Ok((0, 0));
    }
    if !quiet {
        println!("Fetching repository {}", repository.id);
    }
    for submodule in repository.submodules()? {
        let root = submodule.abs_path();
        if let Some(repo) =
            &Repository::try_new(repo_tree_dir, root.clone(), url_parser)?
        {
            let (_ok, _total) =
                fetch_repo(repo_tree_dir, url_parser, quiet, repo, true)?;
            ok += _ok;
            total += _total;
        } else {
            eprintln!("No submodule found in {}", root.display());
        }
    }

    ok += if match repository.vcs {
        VersionControlSystem::Jujutsu | VersionControlSystem::JujutsuGit => {
            if !quiet {
                println!(
                    "Fetching jujutsu {}repository {}",
                    if is_submodule { "submodule " } else { "" },
                    repository.id
                );
            }
            jujutsu::git::fetch(&repository.root, quiet)
        }
        VersionControlSystem::Git => {
            if !quiet {
                println!("Fetching git repository {}", repository.id);
            }
            git::fetch(&repository.root, quiet)
        }
    } == 0
    {
        1
    } else {
        0
    };
    total += 1;

    Ok((ok, total))
}

pub fn run(args: FetchArgs) -> i32 {
    let repo_tree_dir = get_repo_tree_dir();
    let config = Config::default();
    let url_parser = UrlParser::new(&config);
    let (repositories, _) =
        load_repo_tree(&repo_tree_dir, &UrlParser::new(&Config::default()));

    let (ok, total) = repositories
        .iter()
        .map(|r| {
            fetch_repo(&repo_tree_dir, &url_parser, args.quiet, r, false)
                .unwrap_or((0, 1))
        })
        .reduce(|acc, res| (acc.0 + res.0, acc.1 + res.1))
        .unwrap_or((0, 0));

    println!("{ok}/{total} repository fetched");

    if ok == total { 0 } else { 1 }
}
