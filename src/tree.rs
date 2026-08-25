//! Definition of a repository tree.
//! A repository tree is an remote-based organized storage of your repositories.
//! Each tree,
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::fs::create_dir_all;
use std::fs::read_to_string;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::slice::Iter;

use clap::ValueEnum;
use globset::Glob;
use serde::Deserialize;
use serde::Serialize;

use crate::colors::ColoredText;
use crate::config::Config;
use crate::config::TreeCategory;
use crate::error::NoCacheError;
use crate::error::UnexpectedTreeSpaceError;
use crate::error::UnknownRemoteHostError;
use crate::repo_id::RepoId;
use crate::repository::Repository;

/// Tree organization model on how the repositories are organized / stored in
/// the tree-space.
pub enum TreeOrganization<'config> {
    /// Tree-space which contains repositories which have an associated remote.
    /// The repositories are organized based on the default remote URL. To copy
    /// the same organization as on the remote.
    /// The associated string corresponds to the name of the folder to use for
    /// the tree.
    RemoteBased(&'config Config, &'config TreeCategory),
    /// Tree-space contains only local repository which have no
    /// configured
    Local(&'config Config, &'config TreeCategory),
}

impl<'config> TreeOrganization<'config> {
    /// Path to where the directory for that tree-space category is located.
    pub fn location(&self) -> PathBuf {
        let (config, tree_category) = match self {
            Self::RemoteBased(config, tree_category) => (config, tree_category),
            Self::Local(config, tree_category) => (config, tree_category),
        };

        config.root.join(tree_category.dir_name())
    }

    /// Get the expected location of a repository in this tree organization
    /// model.
    pub fn repo_location(
        &self,
        repo_id: &RepoId,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let base = self.location();
        match self {
            Self::RemoteBased(config, tree_category) => {
                if let Some(remote) = &repo_id.remote {
                    Ok(config
                        .get_remote_host(&remote.host_url)
                        .ok_or(UnknownRemoteHostError(
                            remote.host_url.to_string(),
                        ))
                        .map(|remote_host| {
                            base.join(remote_host.category.dir_name()).join(
                                repo_id.name.split('/').collect::<PathBuf>(),
                            )
                        })?)
                } else {
                    Err(Box::new(UnexpectedTreeSpaceError(
                        repo_id.name.clone(),
                        tree_category.name.to_string(),
                    )))
                }
            }
            Self::Local(_, tree_category) => {
                if repo_id.remote.is_some() {
                    Err(Box::new(UnexpectedTreeSpaceError(
                        repo_id.name.clone(),
                        tree_category.name.to_string(),
                    )))
                } else {
                    Ok(base.join(repo_id.name.split('/').collect::<PathBuf>()))
                }
            }
        }
    }

    /// Get the tree-category for this organization.
    /// Note: This method exists so the association TreeSpace <->
    /// TreeOrganization <-> TreeCategory is specified only once in the
    /// TreeSpace::organization() method.
    pub fn category(&self) -> &'config TreeCategory {
        match self {
            Self::RemoteBased(_, tree_category) => tree_category,
            Self::Local(_, tree_category) => tree_category,
        }
    }
}

/// The different repository trees categories.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    ValueEnum,
)]
pub enum TreeSpace {
    /// Main tree, where active, user-modified repository are
    Dev,
    /// Tree for repositories which exists only locally.
    Local,
}

impl TreeSpace {
    /// Obtain the TreeSpace value based on a directory name, directory should
    /// match a tree-category.
    fn from_dir_name(config: &Config, dir_name: &OsStr) -> Option<Self> {
        let tree_config = &config.tree;

        if dir_name == tree_config.dev.category.dir_name() {
            Some(Self::Dev)
        } else if dir_name == tree_config.local.category.dir_name() {
            Some(Self::Local)
        } else {
            None
        }
    }

    /// Obtain the TreeSpace value based on a path which should be inside a
    /// tree-space.
    pub fn from_path(config: &Config, path: &Path) -> Option<Self> {
        Self::from_dir_name(
            config,
            path.strip_prefix(&config.root).ok()?.iter().next()?,
        )
    }

    /// Get the organization model associated with the tree-space.
    fn organization<'config>(
        &self,
        config: &'config Config,
    ) -> TreeOrganization<'config> {
        match self {
            Self::Dev => {
                TreeOrganization::RemoteBased(config, &config.tree.dev.category)
            }
            Self::Local => {
                TreeOrganization::Local(config, &config.tree.local.category)
            }
        }
    }

    /// Get the expected location of a repository in this tree-space.
    pub fn repo_location(
        &self,
        config: &Config,
        repo_id: &RepoId,
    ) -> Result<PathBuf, Box<dyn Error>> {
        self.organization(config).repo_location(repo_id)
    }

    /// Get the representation for this tree-space.
    pub fn repr<'config>(
        &self,
        config: &'config Config,
    ) -> &'config ColoredText {
        &self.organization(config).category().repr
    }
}

/// Search recursively repositories in a directory.
fn _search(
    config: &Config,
    repositories: &mut Vec<Repository>,
    empty_dirs: &mut Vec<PathBuf>,
    dir: &Path,
) {
    if !dir.is_dir() {
        return;
    }

    let mut empty_dir = true;

    for entry in dir.read_dir().expect("read dir call failed").flatten() {
        empty_dir = false;
        let root = entry.path();
        let repo = Repository::try_new(config, &root);

        if let Ok(repo) = repo {
            repositories.push(repo);
        } else {
            _search(config, repositories, empty_dirs, &root);
        }
    }

    if empty_dir {
        empty_dirs.push(dir.to_path_buf());
    }
}

/// Search repositories in the repo tree.
fn search(config: &Config) -> (Vec<Repository>, Vec<PathBuf>) {
    let mut repositories = Vec::new();
    let mut empty_dirs = Vec::new();

    for entry in config
        .root
        .read_dir()
        .expect("read dir call failed")
        .flatten()
    {
        let dir_path = entry.path();
        if TreeSpace::from_dir_name(config, &entry.file_name()).is_some() {
            _search(config, &mut repositories, &mut empty_dirs, &dir_path);
        } else {
            eprintln!(
                "Unexpected tree-space directory: {}",
                dir_path.display()
            );
        }
    }

    (repositories, empty_dirs)
}

/// Representation of the repository tree.
#[derive(Serialize, Deserialize)]
pub struct RepoTree {
    /// List of repositories.
    repositories: Vec<Repository>,
}

impl RepoTree {
    /// Load the repository tree from the cache.
    fn from_cache() -> Result<Self, Box<dyn Error>> {
        let cache_file = cache_file();
        if !cache_file.is_file() {
            Err(Box::new(NoCacheError()))
        } else {
            Ok(toml::from_str::<Self>(&read_to_string(&cache_file)?)?)
        }
    }

    /// Load all the repositories present in the repo tree.
    pub fn load_silent(config: &Config, refresh_cache: bool) -> Self {
        Self::load_silent_with_empty_dirs(config, refresh_cache).0
    }

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
            match Self::from_cache() {
                Ok(repositories) => {
                    if repositories.iter().all(|r| {
                        !r.remote_config.has_been_modified().unwrap_or(true)
                    }) {
                        return (repositories, None);
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Failure to load cache {} {}",
                        cache_file().display(),
                        err
                    );
                }
            }
        }

        eprintln!("Refreshing repositories cache...");

        let (repositories, empty_dirs) = search(config);

        (Self { repositories }, Some(empty_dirs))
    }

    /// Load the repo tree.
    /// Print a warning message if empty directories outside any repository are
    /// found in the repo tree.
    pub fn load(config: &Config, refresh_cache: bool) -> Self {
        let (repositories, empty_dirs) =
            Self::load_silent_with_empty_dirs(config, refresh_cache);

        if let Some(empty_dirs) = empty_dirs {
            for empty_dir in empty_dirs {
                eprintln!(
                    "Empty directory in repo tree: {}",
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
        filter_hosts: &[Glob],
        filter_names: &[Glob],
    ) -> Vec<&'repos Repository> {
        self.repositories
            .iter()
            .filter(|r| {
                (filter_hosts.is_empty()
                    || filter_hosts.iter().any(|host| {
                        match r.id.remote_host(config) {
                            Ok(Some(remote_host)) => host
                                .compile_matcher()
                                .is_match(&remote_host.category.name),
                            Ok(None) => false,
                            Err(err) => {
                                eprintln!("{err}");
                                false
                            }
                        }
                    }))
                    && (filter_names.is_empty()
                        || filter_names.iter().any(|filter_name| {
                            filter_name.compile_matcher().is_match(&r.id.name)
                        }))
            })
            .collect()
    }

    /// Obtain an iterator on the repositories.
    pub fn iter(&self) -> Iter<'_, Repository> {
        self.repositories.iter()
    }
}

/// Path to the repositories cache file.
fn cache_file() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap())
        .join("repo-tree")
        .join("repo-tree.toml")
}

impl Drop for RepoTree {
    fn drop(&mut self) {
        let cache_file = cache_file();

        if let Some(parent) = cache_file.parent()
            && !parent.exists()
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
