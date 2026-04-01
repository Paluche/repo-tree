//! Representation of a repository.
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use pollster::FutureExt;

use crate::Config;
use crate::NotImplementedError;
use crate::RepoId;
use crate::RepoState;
use crate::error::NoRepositoryError;
use crate::git::SubmoduleInfo;
use crate::git::{self};
use crate::jujutsu;
use crate::version_control_system::VersionControlSystem;

#[derive(Debug, Clone, PartialEq)]
/// Representation of a repository.
pub struct Repository {
    /// Type of version control system the repository uses.
    pub vcs: VersionControlSystem,
    /// Boolean indicating if the repository is a git submodule or not.
    pub is_submodule: bool,
    /// Path to the root of the repository.
    pub root: PathBuf,
    /// Identifier of the repository.
    pub id: RepoId,
}

impl Repository {
    /// Search for a repository at the given path.
    pub fn discover(
        config: &Config,
        path: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let mut current_path = Some(path.clone());

        while current_path.is_some() {
            let root = current_path.clone().unwrap();
            match Self::try_new(config, root) {
                Ok(repo) => return Ok(repo),
                Err(err) => {
                    if err.downcast_ref::<NoRepositoryError>().is_none() {
                        return Err(err);
                    }
                }
            }
            current_path =
                current_path.unwrap().parent().map(|p| p.to_path_buf());
        }

        Err(Box::new(NoRepositoryError(path)))
    }

    /// Try loading a repository which root is the one provided.
    pub fn try_new(
        config: &Config,
        root: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let vcs = VersionControlSystem::try_new(&root);
        if vcs.is_none() {
            return Err(Box::new(NoRepositoryError(root)));
        }
        let (vcs, is_submodule) = vcs.unwrap();
        let remote_url = match vcs {
            VersionControlSystem::Git | VersionControlSystem::JujutsuGit => {
                git::get_remote_url(&root)?
            }
            VersionControlSystem::Jujutsu => jujutsu::get_remote_url(&root)?,
        };
        let id = RepoId::parse_repo_url(config, &root, remote_url.as_ref())?;

        Ok(Self {
            vcs,
            is_submodule,
            root,
            id,
        })
    }

    /// Get the expected path to the root of the repository within the repo
    /// tree. If the repository is a submodule then, it has to be at its place
    /// within its main repository and therefore we return None.
    pub fn expected_root(&self, config: &Config) -> Option<PathBuf> {
        if self.is_submodule {
            None
        } else {
            self.id.location(config)
        }
    }

    /// Get the git submodules present in the repository.
    pub fn submodules(&self) -> Result<Vec<SubmoduleInfo>, Box<dyn Error>> {
        Ok(if self.vcs.is_git() {
            git::submodules::get(&self.root, self.id.remote_url.clone())?
        } else {
            Vec::new()
        })
    }

    /// Get the repository state.
    pub fn state(&self) -> Result<RepoState, Box<dyn Error>> {
        Ok(match self.vcs {
            VersionControlSystem::Jujutsu
            | VersionControlSystem::JujutsuGit => {
                jujutsu::get_repo_state(&self.root).block_on()?
            }
            vcs => Err(NotImplementedError(format!(
                "Repository state for {vcs} Version Control"
            )))?,
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

/// Search recursively repositories in a directory.
fn _search(config: &Config, dir: &Path) -> (Vec<Repository>, Vec<PathBuf>) {
    let mut repositories = Vec::new();
    let mut empty_dirs = Vec::new();
    if !dir.is_dir() {
        return (repositories, empty_dirs);
    }

    let mut empty_dir = true;

    for entry in dir.read_dir().expect("read dir call failed").flatten() {
        empty_dir = false;
        let root = entry.path();
        let repo = Repository::try_new(config, root.clone());
        if let Ok(repo) = repo {
            repositories.push(repo);
        } else {
            let res = _search(config, &root);
            repositories.extend(res.0);
            empty_dirs.extend(res.1);
        }
    }

    if empty_dir {
        empty_dirs.push(dir.to_path_buf());
    }

    (repositories, empty_dirs)
}

/// Search repositories in the repo tree.
pub fn search(config: &Config) -> (Vec<Repository>, Vec<PathBuf>) {
    _search(config, &config.repo_tree_dir)
}
