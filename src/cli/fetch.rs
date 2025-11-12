//! Fetch and update the whole repo_tree.

use crate::{
    Config, Repository, UrlParser, VersionControlSystem, get_repo_tree_dir,
    git, jujutsu, load_repo_tree,
};
use std::{error::Error, path::Path};

pub fn fetch_repo(
    repo_tree_dir: &Path,
    url_parser: &UrlParser,
    repository: &Repository,
) -> Result<i32, Box<dyn Error>> {
    if let Some(host) = &repository.id.host
        && host.name == "local"
    {
        println!("Skipping local repository {}", repository.id);
        return Ok(0);
    }
    println!("Fetching repository {}", repository.id);
    for submodule in repository.submodules()? {
        let root = submodule.abs_path();
        if let Some(repo) =
            &Repository::try_new(repo_tree_dir, root.clone(), url_parser)?
        {
            fetch_repo(repo_tree_dir, url_parser, repo)?;
        } else {
            eprintln!("No repository found in {}", root.display());
        }
    }

    Ok(match repository.vcs {
        VersionControlSystem::Jujutsu | VersionControlSystem::JujutsuGit => {
            jujutsu::git::fetch(&repository.root)
        }
        VersionControlSystem::Git => git::fetch(&repository.root),
    })
}

pub fn fetch() -> i32 {
    let repo_tree_dir = get_repo_tree_dir();
    let config = Config::default();
    let url_parser = UrlParser::new(&config);
    let (repositories, _) =
        load_repo_tree(&repo_tree_dir, &UrlParser::new(&Config::default()));

    repositories
        .iter()
        .map(|r| fetch_repo(&repo_tree_dir, &url_parser, r).unwrap_or(1))
        .sum()
}
