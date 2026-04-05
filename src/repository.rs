//! Representation of a repository.
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use std::slice::Iter;

use chrono::DateTime;
use chrono::Utc;
use pollster::FutureExt;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::error::NoRepositoryError;
use crate::error::NotImplementedError;
use crate::error::UnknownRemoteHostError;
use crate::git::SubmoduleInfo;
use crate::git::{self};
use crate::jujutsu;
use crate::repo_id::RepoId;
use crate::repo_state::RepoState;
use crate::utils::get_last_modified;
use crate::version_control_system::VersionControlSystem;

/// Metadata about the file containing the
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct RemoteConfig {
    /// Path to the file containing the remote information.
    file: PathBuf,
    /// Last time the file was modified.
    last_modified: DateTime<Utc>,
}

impl RemoteConfig {
    /// Create a new RemoteConfig structure.
    fn new(file: PathBuf) -> Result<Self, Box<dyn Error>> {
        let last_modified = get_last_modified(&file)?;

        Ok(Self {
            file,
            last_modified,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Representation of a repository.
pub struct Repository<'config> {
    /// Type of version control system the repository uses.
    pub vcs: VersionControlSystem,
    /// Boolean indicating if the repository is a git submodule or not.
    pub is_submodule: bool,
    /// Path to the root of the repository.
    pub root: PathBuf,
    /// Identifier of the repository.
    pub id: RepoId<'config>,
    /// Path to the file containing the remote information.
    remote_config: RemoteConfig,
}

impl<'config> Repository<'config> {
    /// Search for a repository at the given path without printing any warning
    /// about the repository location.
    pub fn discover_silent(
        config: &'config Config,
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

    /// Search for a repository at the given path.
    pub fn discover(
        config: &'config Config,
        path: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let repository = Self::discover_silent(config, path)?;

        if let Some(expected_root) = repository.expected_root(config)?
            && repository.root != expected_root
        {
            eprintln!(
                "⚠️Unexpected location for the repository {}. Currently in \
                 \"{}\" should be in \"{}\". Run `rt clean` to fix it.",
                repository.id.name,
                repository.root.display(),
                expected_root.display(),
            );
        }
        Ok(repository)
    }

    /// Try loading a repository which root is the one provided.
    pub fn try_new(
        config: &'config Config,
        root: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let vcs = VersionControlSystem::try_new(&root);
        if vcs.is_none() {
            return Err(Box::new(NoRepositoryError(root)));
        }
        let (vcs, is_submodule) = vcs.unwrap();
        let (remote_config, remote_url) = match vcs {
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
            remote_config: RemoteConfig::new(remote_config)?,
        })
    }

    /// Get the expected path to the root of the repository within the repo
    /// tree. If the repository is a submodule then, it has to be at its place
    /// within its main repository and therefore we return None.
    pub fn expected_root(
        &self,
        config: &Config,
    ) -> Result<Option<PathBuf>, UnknownRemoteHostError> {
        Ok(if self.is_submodule {
            None
        } else {
            Some(self.id.location(config)?)
        })
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

impl<'config> Display for Repository<'config> {
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
fn _search<'config>(
    config: &'config Config,
    dir: &Path,
) -> (Vec<Repository<'config>>, Vec<PathBuf>) {
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
fn search(config: &Config) -> (Vec<Repository<'_>>, Vec<PathBuf>) {
    _search(config, &config.repo_tree_dir)
}

/// Load all the repositories present in the repo tree.
/// Print a warning message if empty directories outside any repository are
/// found in the repo tree.
fn load_repositories(config: &Config) -> Vec<Repository<'_>> {
    let (repositories, empty_dirs) = search(config);

    for empty_dir in empty_dirs {
        eprintln!("Empty directory in REPO_TREE_DIR: {}", empty_dir.display());
    }

    repositories
}

/// Search for empty directories outside any repository are found in the
/// repo tree.
pub fn load_empty_dirs(config: &Config) -> Vec<PathBuf> {
    search(config).1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Repositories present in the repo tree.
pub struct Repositories<'config> {
    /// The repositories in the repo tree.
    repositories: Vec<Repository<'config>>,
}

impl<'config> Repositories<'config> {
    /// Load all the repositories present in the repo tree.
    pub fn load_silent(config: &'config Config) -> Self {
        Self {
            repositories: search(config).0,
        }
    }

    /// Load all the repositories present in the repo tree.
    /// Print a warning message if empty directories outside any repository are
    /// found in the repo tree.
    pub fn load(config: &'config Config) -> Self {
        let (repositories, empty_dirs) = search(config);

        for empty_dir in empty_dirs {
            eprintln!(
                "Empty directory in REPO_TREE_DIR: {}",
                empty_dir.display()
            );
        }

        Self { repositories }
    }

    /// Load some of the repositories based on the provided filters.
    pub fn load_filtered(
        config: &'config Config,
        filter_hosts: Vec<String>,
        filter_names: Vec<String>,
    ) -> Self {
        let repositories = load_repositories(config)
            .into_iter()
            .filter(|r| {
                (filter_hosts.is_empty()
                    || filter_hosts.iter().any(|host| match r.id.host.name() {
                        Ok(name) => name == host,
                        Err(err) => {
                            eprintln!("{err}");
                            false
                        }
                    }))
                    && (filter_names.is_empty()
                        || filter_names
                            .iter()
                            .any(|name| r.id.name.starts_with(name)))
            })
            .collect();

        Self { repositories }
    }

    /// Iterate the repositories, starting from the specified one.
    pub fn iter_from(
        &'config self,
        start: &'config Option<Repository<'config>>,
        reverse: bool,
    ) -> Box<dyn Iterator<Item = &'config Repository<'config>> + 'config> {
        if let Some(start) = start {
            if reverse {
                Box::new(
                    self.repositories
                        .iter()
                        .cycle()
                        .skip_while(|r| **r != start.clone())
                        .take_while(|r| **r != start.clone()),
                )
            } else {
                Box::new(
                    self.repositories
                        .iter()
                        .rev()
                        .cycle()
                        .skip_while(|r| **r != start.clone())
                        .take_while(|r| **r != start.clone()),
                )
            }
        } else if reverse {
            Box::new(self.repositories.iter().rev())
        } else {
            Box::new(self.repositories.iter())
        }
    }

    /// Obtain an iterator on the repositories.
    pub fn iter(&self) -> Iter<'_, Repository<'_>> {
        self.repositories.iter()
    }
}
