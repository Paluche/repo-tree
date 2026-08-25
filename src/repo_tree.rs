//! Definition of a repository tree. A repository tree is an remote-based
//! organized storage of your repositories. Each tree,
use std::error::Error;
use std::fs::File;
use std::fs::create_dir_all;
use std::fs::read_to_string;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

use globset::Glob;
use serde::Deserialize;
use serde::Serialize;
use strum::IntoEnumIterator;

use crate::config::Config;
use crate::error::NoCacheError;
use crate::repository::Repository;
use crate::repository::Workspace;
use crate::tree_space::TreeSpace;
use crate::tree_space::TreeSpaceKind;

/// Search recursively repositories in a directory.
fn _search(
    config: &Config,
    tree_space_kind: &TreeSpaceKind,
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
            if tree_space_kind.is_workspace() {
                if let Some(main_repo) =
                    repositories.iter_mut().find(|r| r.id == repo.id)
                {
                } else {
                    eprintln!(
                        "Unexpected workspace repository without a known \
                         default associated in {}",
                        root.display()
                    );
                    continue;
                }
            } else {
                repositories.push(repo);
            }
        } else {
            _search(config, tree_space_kind, repositories, empty_dirs, &root);
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
        if let Some(tree_space) =
            TreeSpace::from_dir_name(config, &entry.file_name())
        {
            if tree_space.kind().is_workspace() {
                continue;
            }
            _search(
                config,
                &tree_space.kind(),
                &mut repositories,
                &mut empty_dirs,
                &dir_path,
            );
        } else {
            eprintln!(
                "Unexpected tree-space directory: {}",
                dir_path.display()
            );
        }
    }

    for tree_space in
        TreeSpace::iter().filter(|tree_space| tree_space.kind().is_workspace())
    {
        let dir_path = config.root.join(tree_space.category(config).dir_name());
        _search(
            config,
            &tree_space.kind(),
            &mut repositories,
            &mut empty_dirs,
            &dir_path,
        );
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
                Ok(repo_tree) => {
                    if repo_tree.repo_iter().all(|r| {
                        !r.remote_config.has_been_modified().unwrap_or(true)
                    }) {
                        return (repo_tree, None);
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
    pub fn filtered<'repo_tree>(
        &'repo_tree self,
        config: &Config,
        filter_hosts: &[Glob],
        filter_names: &[Glob],
        filter_trees: &[Glob],
    ) -> Vec<(&'repo_tree Repository, &'repo_tree Workspace)> {
        self.workspace_iter()
            .filter(|(r, w)| {
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
                    && (filter_trees.is_empty()
                        || filter_trees.iter().any(|filter_tree| {
                            if let Some(tree_space) = &w.tree_space() {
                                filter_tree
                                    .compile_matcher()
                                    .is_match(&tree_space.category(config).name)
                            } else {
                                false
                            }
                        }))
            })
            .collect()
    }

    /// Obtain an iterator on all the repositories.
    pub fn repo_iter<'repo_tree>(
        &'repo_tree self,
    ) -> Box<dyn Iterator<Item = &'repo_tree Repository> + 'repo_tree> {
        Box::new(self.repositories.iter())
    }
    /// Obtain an iterator on all the repositories workspaces.
    pub fn workspace_iter<'repo_tree>(
        &'repo_tree self,
    ) -> Box<
        dyn Iterator<Item = (&'repo_tree Repository, &'repo_tree Workspace)>
            + 'repo_tree,
    > {
        Box::new(
            self.repositories
                .iter()
                .flat_map(|r| r.workspaces.iter().map(move |w| (r, w))),
        )
    }
}

/// Path to the repositories cache file.
fn cache_file() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache")
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
