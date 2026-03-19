//! Representation of a repository.
use std::error::Error;
use std::fs::File;
use std::fs::create_dir_all;
use std::fs::read_to_string;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::slice::Iter;

use chrono::DateTime;
use chrono::Utc;
use pollster::FutureExt;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::error::NoCacheError;
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
#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

    /// Does the file have been modified compared to the last_modified value we
    /// have.
    fn has_been_modified(&self) -> Result<bool, Box<dyn Error>> {
        Ok(self.last_modified != get_last_modified(&self.file)?)
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
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
    /// Path to the file containing the remote information.
    remote_config: RemoteConfig,
}

impl Repository {
    /// Search for a repository at the given path without printing any warning
    /// about the repository location.
    pub fn discover_silent(
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

    /// Search for a repository at the given path.
    pub fn discover(
        config: &Config,
        path: PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        let repository = Self::discover_silent(config, path)?;

        if let Some(expected_root) = repository.expected_root(config)?
            && repository.root != expected_root
            && !config.should_be_ignored(&repository.root)
        {
            eprintln!(
                "⚠️Unexpected location for the repository {}. Currently in \
                 \"{}\" should be in \"{}\". Run `{}` to fix it.",
                repository.id.name,
                repository.root.display(),
                expected_root.display(),
                if repository.root.starts_with(&config.repo_tree_dir) {
                    "rt clean".to_string()
                } else {
                    format!("rt insert \"{}\"", repository.root.display())
                }
            );
        }
        Ok(repository)
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
        let (remote_config, remote_url) = match vcs {
            VersionControlSystem::Git | VersionControlSystem::JujutsuGit => {
                git::get_remote_url(&root)?
            }
            VersionControlSystem::Jujutsu => jujutsu::get_remote_url(&root)?,
        };
        let id = RepoId::from_repo(config, &root, remote_url.as_ref())?;

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
            git::submodules::get(&self.root, &self.id.remote)?
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
fn search(config: &Config) -> (Vec<Repository>, Vec<PathBuf>) {
    _search(config, &config.repo_tree_dir)
}

/// Path to the repositories cache file.
fn cache_file() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap())
        .join("repo-tree")
        .join("repositories.toml")
}

/// Load the repository list from the cache.
fn load_cache() -> Result<Repositories, Box<dyn Error>> {
    let cache_file = cache_file();
    if !cache_file.is_file() {
        Err(Box::new(NoCacheError()))
    } else {
        Ok(toml::from_str::<Repositories>(&read_to_string(
            &cache_file,
        )?)?)
    }
}

#[derive(Clone, Serialize, Deserialize)]
/// Repositories present in the repo tree.
pub struct Repositories {
    /// The repositories in the repo tree.
    repositories: Vec<Repository>,
}

impl Repositories {
    /// Load all the repositories present in the repo tree with a list of
    /// detected empty directories within the repo tree. The list of empty
    /// directories, is returned only if the cache has not been used. As the
    /// cache exists to avoids us searching the repo tree, we should not do it
    /// anyway for getting the empty directories.
    pub fn load_silent_with_empty_dirs(
        config: &Config,
        refresh_cache: bool,
    ) -> (Self, Option<Vec<PathBuf>>) {
        if !refresh_cache {
            match load_cache() {
                Ok(repositories) => {
                    if repositories.iter().all(|r| {
                        !r.remote_config.has_been_modified().unwrap_or(true)
                    }) {
                        return (repositories, None);
                    }
                }
                Err(err) => {
                    eprintln!("Failure to load cache {err}");
                }
            }
        }

        eprintln!("Refreshing repositories cache...");

        let (repositories, empty_dirs) = search(config);

        (Self { repositories }, Some(empty_dirs))
    }

    /// Load all the repositories present in the repo tree.
    pub fn load_silent(config: &Config, refresh_cache: bool) -> Self {
        Self::load_silent_with_empty_dirs(config, refresh_cache).0
    }

    /// Load all the repositories present in the repo tree.
    /// Print a warning message if empty directories outside any repository are
    /// found in the repo tree.
    pub fn load(config: &Config, refresh_cache: bool) -> Self {
        let (repositories, empty_dirs) =
            Self::load_silent_with_empty_dirs(config, refresh_cache);

        if let Some(empty_dirs) = empty_dirs {
            for empty_dir in empty_dirs {
                eprintln!(
                    "Empty directory in REPO_TREE_DIR: {}",
                    empty_dir.display()
                );
            }
        }

        repositories
    }

    /// Load some of the repositories based on the provided filters.
    pub fn filtered<'repos>(
        &'repos self,
        config: &Config,
        filter_hosts: Vec<String>,
        filter_names: Vec<String>,
    ) -> Vec<&'repos Repository> {
        self.repositories
            .iter()
            .filter(|r| {
                (filter_hosts.is_empty()
                    || filter_hosts.iter().any(|host| {
                        match r.id.remote.host(config).name() {
                            Ok(name) => name == host,
                            Err(err) => {
                                eprintln!("{err}");
                                false
                            }
                        }
                    }))
                    && (filter_names.is_empty()
                        || filter_names
                            .iter()
                            .any(|name| r.id.name.starts_with(name)))
            })
            .collect()
    }

    /// Obtain an iterator on the repositories.
    pub fn iter(&self) -> Iter<'_, Repository> {
        self.repositories.iter()
    }
}

impl Drop for Repositories {
    fn drop(&mut self) {
        let cache_file = cache_file();
        let parent = cache_file.parent().unwrap();

        if !parent.exists()
            && let Err(err) = create_dir_all(parent)
        {
            eprintln!(
                "Unable to create cache directory \"{}\": {err}",
                parent.display()
            );
        }
        if let Err(err) = File::create(cache_file)
            .map(|mut f| f.write_all(toml::to_string(self).unwrap().as_bytes()))
        {
            eprintln!("Unable to create cache file: {err}");
        }
    }
}
