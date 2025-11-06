//! Fetch and update the whole workspace.

use crate::{
    Config, Repository, UrlParser, VersionControlSystem, get_workspace_dir,
    git, jujutsu, load_workspace,
};
use std::{error::Error, path::Path};

pub fn fetch_repo(
    workspace_dir: &Path,
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
    for (submodule, _, _) in repository.submodules()? {
        let root = repository.root.join(&submodule);
        if let Some(repo) =
            &Repository::try_new(workspace_dir, root.clone(), url_parser)?
        {
            fetch_repo(workspace_dir, url_parser, repo)?;
        } else {
            eprintln!("No repository found in {}", root.display());
        }
    }

    let ret = match repository.vcs {
        VersionControlSystem::Jujutsu | VersionControlSystem::JujutsuGit => {
            jujutsu::git::fetch(&repository.root)
        }
        VersionControlSystem::Git => git::fetch(&repository.root),
    };

    // Git delete-branch?

    if repository.is_submodule {
        return Ok(ret);
    }

    Ok(0)
}

pub fn fetch() -> i32 {
    let workspace_dir = get_workspace_dir();
    let config = Config::default();
    let url_parser = UrlParser::new(&config);
    let (repositories, _) =
        load_workspace(&workspace_dir, &UrlParser::new(&Config::default()));

    repositories
        .iter()
        .map(|r| fetch_repo(&workspace_dir, &url_parser, r).unwrap_or(1))
        .sum()
}
