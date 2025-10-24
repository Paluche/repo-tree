//! Representation of a repository.
use crate::version_control_system::VersionControlSystem;
use crate::{UrlParser, git, jujutsu};
use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Repository {
    pub vcs: VersionControlSystem,
    pub is_submodule: bool,
    pub root: PathBuf,
    pub remote_url: Option<String>,
    pub forge: Option<String>,
    pub name: String,
}

/// Either the repository is within the ${WORK_DIR}/local directory
/// allowing the user to organize as see fits this directory.
/// Or take the directory name.
fn compute_local_path<P: AsRef<Path>>(
    work_dir: &Path,
    repo_path: &P,
) -> String {
    let local_dir = work_dir.join("local");
    let repo_path = repo_path.as_ref();
    assert!(repo_path.is_absolute(), "repo_path is not absolute");
    assert!(local_dir.is_absolute(), "local_dir is not absolute");

    if repo_path.starts_with(&local_dir) {
        repo_path
            .iter()
            .skip(local_dir.iter().count())
            .collect::<PathBuf>()
            .display()
            .to_string()
    } else {
        repo_path.file_name().unwrap().to_str().unwrap().to_owned()
    }
}

fn compute_repo_forge_name<P: AsRef<Path>>(
    work_dir: &Path,
    url_parser: &UrlParser,
    remote_url: Option<&String>,
    repo_path: &P,
) -> (Option<String>, String) {
    url_parser.parse(remote_url).unwrap_or((
        Some("local".to_string()),
        compute_local_path(work_dir, repo_path),
    ))
}

impl Repository {
    pub fn discover(
        work_dir: &Path,
        path: PathBuf,
        url_parser: &UrlParser,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        let mut current_path = Some(path);

        while current_path.is_some() {
            if let Some(repo) = Self::try_new(
                work_dir,
                current_path.clone().unwrap(),
                url_parser,
            )? {
                return Ok(Some(repo));
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
        let (forge, name) = compute_repo_forge_name(
            work_dir,
            url_parser,
            remote_url.as_ref(),
            &root,
        );

        Ok(Some(Self {
            vcs,
            is_submodule,
            root,
            remote_url,
            forge,
            name,
        }))
    }

    pub fn expected_root(&self, work_dir: &Path) -> Option<PathBuf> {
        if self.is_submodule || self.forge.is_none() {
            None
        } else {
            let mut path = work_dir.to_path_buf();
            path.push(self.forge.clone().unwrap());
            path.push(&self.name);
            Some(path)
        }
    }
}

impl Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} repository at {} ({:?} {} {:?})",
            self.vcs,
            self.root.display(),
            self.forge,
            self.name,
            self.remote_url
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
