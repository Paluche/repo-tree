//! Representation of a repository.
use crate::{
    UrlParser,
    config::Host,
    git::{self, SubmoduleInfo},
    jujutsu,
    version_control_system::VersionControlSystem,
};
use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Hash)]
pub struct RepoId {
    pub remote_url: Option<String>,
    pub host: Option<Host>,
    pub name: String,
}

impl RepoId {
    pub fn expected_root(&self, work_dir: &Path) -> Option<PathBuf> {
        self.host.clone().map(|host| {
            let mut path = work_dir.to_path_buf();
            path.push(host.name);
            path.push(&self.name);
            path
        })
    }
}

impl Display for RepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {} {:?})", self.host, self.name, self.remote_url)
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
        work_dir: &Path,
        path: PathBuf,
        url_parser: &UrlParser,
    ) -> Result<Option<(PathBuf, Self)>, Box<dyn Error>> {
        let mut current_path = Some(path);

        while current_path.is_some() {
            let root = current_path.clone().unwrap();
            if let Some(repo) =
                Self::try_new(work_dir, root.clone(), url_parser)?
            {
                return Ok(Some((root, repo)));
            }
            current_path =
                current_path.unwrap().parent().map(|p| p.to_path_buf());
        }

        Ok(None)
    }

    pub fn try_new(
        work_dir: &Path,
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
            //VersionControlSystem::Subversion => {
            //}
            _ => None,
        };
        let (host, name) =
            url_parser.parse(work_dir, remote_url.as_ref(), &root);

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

    pub fn expected_root(&self, work_dir: &Path) -> Option<PathBuf> {
        if self.is_submodule {
            None
        } else {
            self.id.expected_root(work_dir)
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
    work_dir: &Path,
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
            Repository::try_new(work_dir, root.clone(), url_parser).unwrap()
        {
            repositories.push(repo);
        } else {
            let res = _search(work_dir, &root, url_parser);
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
    work_dir: &Path,
    url_parser: &UrlParser,
) -> (Vec<Repository>, Vec<PathBuf>) {
    _search(work_dir, work_dir, url_parser)
}
