//! Representation of a repository.
//! TODO
//! Load of the associated remote url to associate a repository ID.
use crate::version_control_system::VersionControlSystem;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

pub struct Repository {
    vcs: VersionControlSystem,
    root: PathBuf,
}

impl Repository {
    pub fn try_new(root: PathBuf) -> Option<Self> {
        let vcs = VersionControlSystem::try_new(&root)?;

        Some(Self { vcs, root })
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
        write!(f, "{} repository at {}", self.vcs, self.root.display())
    }
}

pub fn search(dir: &Path) -> Vec<Repository> {
    let mut ret = Vec::new();
    if !dir.is_dir() {
        return ret;
    }

    for entry in dir.read_dir().expect("read dir call failed").flatten() {
        let root = entry.path();
        if let Some(repo) = Repository::try_new(root.clone()) {
            ret.push(repo);
        } else {
            ret.extend(search(&root));
        }
    }

    ret
}
