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
use crate::error::NoRepositoryError;
use crate::repository::Repository;
use crate::repository::Workspace;
use crate::tree_space::TreeSpace;
use crate::tree_space::TreeSpaceKind;
use crate::ui::Ui;
use crate::ui_error;
use crate::ui_warning;

/// Search recursively repositories in a directory.
fn search_dir(
    ui: &Ui<'_>,
    config: &Config,
    tree_space_kind: &TreeSpaceKind,
    repositories: &mut Vec<Repository>,
    empty_dirs: &mut Vec<PathBuf>,
    dir: &Path,
) {
    if !dir.is_dir() {
        return;
    }

    let entries = match dir.read_dir() {
        Ok(entries) => entries,
        Err(err) => {
            ui_error!(
                ui,
                "Unable to read the directory {}: {err}",
                dir.display()
            );
            return;
        }
    };

    let mut empty_dir = true;

    for entry in entries.flatten() {
        empty_dir = false;
        let root = entry.path();

        match Repository::try_new(config, &root) {
            Ok(repo) => {
                if !tree_space_kind.is_workspace() {
                    repositories.push(repo);
                } else if let Some(main_repo) =
                    repositories.iter_mut().find(|r| r.id == repo.id)
                {
                    main_repo.add_workspace(repo);
                } else {
                    ui_warning!(
                        ui,
                        "Unexpected workspace repository without a known \
                         default associated in {}",
                        root.display()
                    );
                }
            }
            Err(err) if err.downcast_ref::<NoRepositoryError>().is_some() => {
                search_dir(
                    ui,
                    config,
                    tree_space_kind,
                    repositories,
                    empty_dirs,
                    &root,
                );
            }
            // A repository which cannot be loaded is not to be explored as a
            // directory of the tree.
            Err(err) => ui_error!(
                ui,
                "Unable to load the repository {}: {err}",
                root.display()
            ),
        }
    }

    if empty_dir {
        empty_dirs.push(dir.to_path_buf());
    }
}

/// Search repositories in the repo tree.
fn search(ui: &Ui<'_>, config: &Config) -> (Vec<Repository>, Vec<PathBuf>) {
    let mut repositories = Vec::new();
    let mut empty_dirs = Vec::new();

    let entries = match config.root.read_dir() {
        Ok(entries) => entries,
        Err(err) => {
            ui_error!(
                ui,
                "Unable to read the repo tree root {}: {err}",
                config.root.display()
            );
            return (repositories, empty_dirs);
        }
    };

    for entry in entries.flatten() {
        let dir_path = entry.path();
        if let Some(tree_space) =
            TreeSpace::from_dir_name(config, &entry.file_name())
        {
            if tree_space.kind().is_workspace() {
                continue;
            }
            search_dir(
                ui,
                config,
                &tree_space.kind(),
                &mut repositories,
                &mut empty_dirs,
                &dir_path,
            );
        } else {
            ui_warning!(
                ui,
                "Unexpected tree-space directory: {}",
                dir_path.display(),
            );
        }
    }

    for tree_space in
        TreeSpace::iter().filter(|tree_space| tree_space.kind().is_workspace())
    {
        let dir_path = config.root.join(tree_space.category(config).dir_name());
        search_dir(
            ui,
            config,
            &tree_space.kind(),
            &mut repositories,
            &mut empty_dirs,
            &dir_path,
        );
    }

    for repository in &repositories {
        forget_missing_workspaces(ui, config, repository);
    }

    (repositories, empty_dirs)
}

/// Forget the workspaces of the repository which directory has been deleted
/// without telling the version control system, so that both agree on the list
/// of workspaces.
fn forget_missing_workspaces(
    ui: &Ui<'_>,
    config: &Config,
    repository: &Repository,
) {
    let Some(workspace) = repository.workspaces.first() else {
        return;
    };

    match repository
        .get_vcs_repo(workspace)
        .forget_missing_workspaces()
    {
        Ok(forgotten) => {
            for w in forgotten {
                ui_warning!(
                    ui,
                    "Forgot the workspace {} of the repository {}: its \
                     directory {} does not exist anymore",
                    w.name,
                    repository.id.display(config),
                    w.path.display()
                );
            }
        }
        Err(err) => {
            ui_error!(
                ui,
                "Unable to forget the missing workspaces of the repository \
                 {}: {}",
                repository.id.display(config),
                err
            );
        }
    }
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
    pub fn load_silent(
        ui: &Ui<'_>,
        config: &Config,
        refresh_cache: bool,
    ) -> Self {
        Self::load_silent_with_empty_dirs(ui, config, refresh_cache).0
    }

    /// Load all the repositories present in the repo tree with a list of
    /// detected empty directories within the repo tree. The list of empty
    /// directories, is returned only if the cache has not been used. As the
    /// cache exists to avoids us searching the repo tree, we should not do it
    /// anyway for getting the empty directories.
    pub fn load_silent_with_empty_dirs(
        ui: &Ui<'_>,
        config: &Config,
        refresh_cache: bool,
    ) -> (Self, Option<Vec<PathBuf>>) {
        if !refresh_cache {
            match Self::from_cache() {
                Ok(repo_tree) => {
                    if !repo_tree.repo_iter().any(|r| r.has_been_modified()) {
                        return (repo_tree, None);
                    }
                }
                Err(err) => {
                    ui_error!(
                        ui,
                        "Failure to load cache {} {}",
                        cache_file().display(),
                        err
                    );
                }
            }
        }

        ui.hint("Refreshing repositories cache...");

        let (repositories, empty_dirs) = search(ui, config);
        let repo_tree = Self { repositories };

        if let Err(err) = repo_tree.save() {
            ui_error!(
                ui,
                "Unable to write the cache {}: {}",
                cache_file().display(),
                err
            );
        }

        (repo_tree, Some(empty_dirs))
    }

    /// Load the repo tree.
    /// Print a warning message if empty directories outside any repository are
    /// found in the repo tree.
    pub fn load(ui: &Ui<'_>, config: &Config, refresh_cache: bool) -> Self {
        let (repositories, empty_dirs) =
            Self::load_silent_with_empty_dirs(ui, config, refresh_cache);

        if let Some(empty_dirs) = empty_dirs {
            for empty_dir in empty_dirs {
                ui_warning!(
                    ui,
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
        ui: &Ui<'_>,
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
                                ui.error(err);
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

    /// Obtain the Repository and Workspace of the repository workspace at the
    /// provided path.
    pub fn get_workspace(
        &self,
        workspace_path: &Path,
    ) -> Option<(&Repository, &Workspace)> {
        self.workspace_iter()
            .find(|(_, w)| w.path() == workspace_path)
    }
}

/// Path to the repositories cache file.
fn cache_file() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache")
        .join("repo-tree")
        .join("repo-tree.toml")
}

impl RepoTree {
    /// Write the repository tree into the cache file.
    fn save(&self) -> Result<(), Box<dyn Error>> {
        let cache_file = cache_file();

        if let Some(parent) = cache_file.parent() {
            create_dir_all(parent)?;
        }

        File::create(&cache_file)?
            .write_all(toml::to_string(self)?.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
impl RepoTree {
    /// Create a repository tree for tests from its repositories.
    pub fn from_repositories(repositories: Vec<Repository>) -> Self {
        Self { repositories }
    }
}
