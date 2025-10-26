mod status;

pub use status::{GitStatus, SubmoduleStatus, status};
use std::path::{Path, PathBuf};

pub fn get_remote_url_repo(
    repo: &git2::Repository,
) -> Result<Option<String>, git2::Error> {
    Ok(repo
        .find_remote("origin")
        .map_or(
            match repo.remotes()?.get(0) {
                Some(name) => Some(repo.find_remote(name)?),
                None => None,
            },
            Some,
        )
        .and_then(|r| r.url().map(String::from)))
}

pub fn get_remote_url<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Option<String>, git2::Error> {
    let repo = git2::Repository::discover(repo_path)?;

    get_remote_url_repo(&repo)
}

pub fn get_submodules<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Vec<(PathBuf, Option<String>)>, git2::Error> {
    let repo = git2::Repository::discover(repo_path)?;
    let submodules = repo.submodules()?;

    Ok(submodules
        .iter()
        .map(|s| (s.path().to_path_buf(), s.url().map(|s| s.to_string())))
        .collect())
}
