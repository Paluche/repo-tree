//! Representation of a repository.
use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
};

use colored::Colorize;

use crate::{
    UrlParser,
    config::Host,
    git::{self, SubmoduleInfo},
    jujutsu,
    version_control_system::VersionControlSystem,
};

#[derive(Debug, Clone, Hash)]
pub struct RepoId {
    pub remote_url: Option<String>,
    pub host: Option<Host>,
    pub name: String,
}

pub fn location(repo_tree_dir: &Path, host: &Host, name: &String) -> PathBuf {
    repo_tree_dir.to_path_buf().join(&host.dir_name).join(name)
}

impl RepoId {
    pub fn expected_root(&self, repo_tree_dir: &Path) -> Option<PathBuf> {
        self.host
            .clone()
            .map(|host| location(repo_tree_dir, &host, &self.name))
    }
}

impl Display for RepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(host) = &self.host
            && host.dir_name != "."
        {
            write!(f, "{} ", host.dir_name)?;
        }
        write!(f, "{}", self.name)?;
        if let Some(remote_url) = &self.remote_url {
            write!(f, " {remote_url}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub vcs: VersionControlSystem,
    pub is_submodule: bool,
    pub root: PathBuf,
    pub id: RepoId,
}

impl Repository {
    pub fn discover(
        repo_tree_dir: &Path,
        path: PathBuf,
        url_parser: &UrlParser,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        let mut current_path = Some(path);

        while current_path.is_some() {
            let root = current_path.clone().unwrap();
            if let Some(repo) = Self::try_new(repo_tree_dir, root, url_parser)?
            {
                return Ok(Some(repo));
            }
            current_path =
                current_path.unwrap().parent().map(|p| p.to_path_buf());
        }

        Ok(None)
    }

    pub fn try_new(
        repo_tree_dir: &Path,
        root: PathBuf,
        url_parser: &UrlParser,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        let vcs = VersionControlSystem::try_new(&root);
        if vcs.is_none() {
            return Ok(None);
        }
        let (vcs, is_submodule) = vcs.unwrap();
        let remote_url = match vcs {
            VersionControlSystem::Git | VersionControlSystem::JujutsuGit => {
                git::get_remote_url(&root)?
            }
            VersionControlSystem::Jujutsu => jujutsu::get_remote_url(&root)?,
        };
        let (host, name) = url_parser.parse_repo_url(
            repo_tree_dir,
            &root,
            remote_url.as_ref(),
        );

        let id = RepoId {
            remote_url,
            host,
            name,
        };

        Ok(Some(Self {
            vcs,
            is_submodule,
            root,
            id,
        }))
    }

    pub fn expected_root(&self, repo_tree_dir: &Path) -> Option<PathBuf> {
        if self.is_submodule {
            None
        } else {
            self.id.expected_root(repo_tree_dir)
        }
    }

    pub fn submodules(&self) -> Result<Vec<SubmoduleInfo>, Box<dyn Error>> {
        Ok(if self.vcs.is_git() {
            git::submodules::get(&self.root, self.id.remote_url.clone())?
        } else {
            Vec::new()
        })
    }
}

impl Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} repository at {} ({})",
            self.vcs,
            self.root.display(),
            self.id,
        )
    }
}

fn _search(
    repo_tree_dir: &Path,
    dir: &Path,
    url_parser: &UrlParser,
) -> (Vec<Repository>, Vec<PathBuf>) {
    let mut repositories = Vec::new();
    let mut empty_dirs = Vec::new();
    if !dir.is_dir() {
        return (repositories, empty_dirs);
    }

    let mut empty_dir = true;

    for entry in dir.read_dir().expect("read dir call failed").flatten() {
        empty_dir = false;
        let root = entry.path();
        if let Some(repo) =
            Repository::try_new(repo_tree_dir, root.clone(), url_parser)
                .unwrap()
        {
            repositories.push(repo);
        } else {
            let res = _search(repo_tree_dir, &root, url_parser);
            repositories.extend(res.0);
            empty_dirs.extend(res.1);
        }
    }

    if empty_dir {
        empty_dirs.push(dir.to_path_buf());
    }

    (repositories, empty_dirs)
}

pub fn search(
    repo_tree_dir: &Path,
    url_parser: &UrlParser,
) -> (Vec<Repository>, Vec<PathBuf>) {
    _search(repo_tree_dir, repo_tree_dir, url_parser)
}

pub struct RepoState {
    pub unpushed_commits: bool,
    pub needs_restack: bool,
    pub has_conflicts: bool,
}

impl RepoState {
    #[expect(unused)]
    pub fn is_ok(&self) -> bool {
        !(self.unpushed_commits || self.needs_restack || self.has_conflicts)
    }
}

impl Display for RepoState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut empty = true;
        if self.unpushed_commits {
            write!(f, "{}", "unpushed commits".purple())?;
            empty = false;
        }

        if self.needs_restack {
            if !empty {
                write!(f, ", ")?;
            }
            write!(f, "{}", "needs restack".yellow())?;
            empty = false;
        }

        if self.has_conflicts {
            if !empty {
                write!(f, ", ")?;
            }
            write!(f, "{}", "has_conflicts".bright_red())?;
            empty = false;
        }

        if empty {
            write!(f, "{}", "OK".green())?;
        }

        Ok(())
    }
}
