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
    pub root: PathBuf,
    pub remote_url: Option<String>,
    pub forge: Option<String>,
    pub name: String,
}

impl Repository {
    pub fn try_new(
        root: PathBuf,
        url_parser: &UrlParser,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        let vcs = VersionControlSystem::try_new(&root);
        if vcs.is_none() {
            return Ok(None);
        }
        let vcs = vcs.unwrap();
        let remote_url = match vcs {
            VersionControlSystem::Git | VersionControlSystem::JujutsuGit => {
                git::get_remote_url(&root)?
            }
            VersionControlSystem::Jujutsu => jujutsu::get_remote_url(&root)?,
            //VersionControlSystem::Subversion => {
            //}
            _ => None,
        };
        let (forge, name) = url_parser.parse(remote_url.as_ref(), &root);

        Ok(Some(Self {
            vcs,
            root,
            remote_url,
            forge,
            name,
        }))
    }

    pub fn vcs(&self) -> &VersionControlSystem {
        &self.vcs
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
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

pub fn search(dir: &Path, url_parser: &UrlParser) -> Vec<Repository> {
    let mut ret = Vec::new();
    if !dir.is_dir() {
        return ret;
    }

    for entry in dir.read_dir().expect("read dir call failed").flatten() {
        let root = entry.path();
        if let Some(repo) =
            Repository::try_new(root.clone(), url_parser).unwrap()
        {
            ret.push(repo);
        } else {
            ret.extend(search(&root, url_parser));
        }
    }

    ret
}
