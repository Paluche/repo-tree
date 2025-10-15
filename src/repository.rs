//! Representation of a repository.
use crate::version_control_system::VersionControlSystem;
use crate::{git, jujutsu, url_parsing::parse_repo_url};
use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
};

pub struct Repository {
    vcs: VersionControlSystem,
    root: PathBuf,
    remote_url: Option<String>,
    forge: Option<String>,
    name: String,
}

impl Repository {
    pub fn try_new(root: PathBuf) -> Result<Option<Self>, Box<dyn Error>> {
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
        let (forge, name) = parse_repo_url(remote_url.as_ref(), &root);

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

pub fn search(dir: &Path) -> Vec<Repository> {
    let mut ret = Vec::new();
    if !dir.is_dir() {
        return ret;
    }

    for entry in dir.read_dir().expect("read dir call failed").flatten() {
        let root = entry.path();
        if let Some(repo) = Repository::try_new(root.clone()).unwrap() {
            ret.push(repo);
        } else {
            ret.extend(search(&root));
        }
    }

    ret
}
