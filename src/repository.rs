//! Representation of a repository.
use std::error::Error;
use std::fs::remove_dir;
use std::fs::remove_dir_all;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use jiff::Timestamp;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::error::NoRepositoryError;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repo_id::RepoId;
use crate::tree_space::TreeSpace;
use crate::tree_space::TreeSpaceKind;
use crate::ui::Ui;
use crate::ui_warning;
use crate::utils::get_last_modified;
use crate::version_control_system::VcsRepository;
use crate::version_control_system::VersionControlSystem;
use crate::version_control_system::git::SubmoduleInfo;
use crate::version_control_system::git::{self};

/// Metadata about a file which, when modified, invalidates the cached
/// information about the repository.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackedFile {
    /// Path to the file.
    file: PathBuf,
    /// Last time the file was modified.
    last_modified: Timestamp,
}

impl TrackedFile {
    /// Create a new `TrackedFile` structure.
    fn new(file: PathBuf) -> Result<Self, Box<dyn Error>> {
        let last_modified = get_last_modified(&file)?;

        Ok(Self {
            file,
            last_modified,
        })
    }

    /// Does the file have been modified compared to the last_modified value we
    /// have.
    pub fn has_been_modified(&self) -> Result<bool, Box<dyn Error>> {
        Ok(self.last_modified != get_last_modified(&self.file)?)
    }
}

/// A repository workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Workspace {
    /// Workspace located within a known tree-space.
    Tree(TreeSpace, PathBuf),
    /// Workspace located outside a known tree-space.
    Other(PathBuf),
}

impl Workspace {
    /// Create a new Workspace.
    fn new(maybe_tree_space: Option<TreeSpace>, path: &Path) -> Self {
        match maybe_tree_space {
            Some(tree_space) => Self::Tree(tree_space, path.to_path_buf()),
            None => Self::Other(path.to_path_buf()),
        }
    }

    /// Is the workspace located within a known tree-space?
    fn is_tree_workspace(&self, tree_space: &TreeSpace) -> bool {
        match self {
            Self::Tree(t, _) => t == tree_space,
            Self::Other(_) => false,
        }
    }

    /// Is the workspace located within a main kind tree-space?
    fn is_main_workspace(&self) -> bool {
        match self {
            Self::Tree(tree_space, _) => {
                matches!(tree_space.kind(), TreeSpaceKind::Main)
            }
            Self::Other(_) => true,
        }
    }

    /// Get the tree-space associated with the workspace if applicable.
    pub fn tree_space(&self) -> Option<&TreeSpace> {
        match self {
            Self::Tree(tree_space, _) => Some(tree_space),
            Self::Other(_) => None,
        }
    }

    /// Get the path to the workspace.
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Tree(_, path) => path,
            Self::Other(path) => path,
        }
    }

    /// Remove the workspace directory, then its parent directories left empty,
    /// up to the tree-space directory containing it (or the repo tree root for
    /// a workspace outside any tree-space).
    pub fn rm(&self, config: &Config) -> io::Result<()> {
        let root = self.path();
        remove_dir_all(root)?;

        let stop = match self {
            Self::Tree(tree_space, _) => {
                config.root.join(tree_space.category(config).dir_name())
            }
            Self::Other(_) => config.root.clone(),
        };

        remove_empty_parents(root, &stop)?;

        Ok(())
    }
}

/// Remove the parent directories of `path` left empty, up to `stop`, which is
/// kept. Nothing is removed if `path` is not within `stop`. Returns the
/// removed directories.
pub fn remove_empty_parents(
    path: &Path,
    stop: &Path,
) -> io::Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    let mut dir = path.parent();
    while let Some(current) = dir
        && current.starts_with(stop)
        && current != stop
    {
        if current.read_dir()?.next().is_some() {
            break;
        }
        remove_dir(current)?;
        removed.push(current.to_path_buf());
        dir = current.parent();
    }

    Ok(removed)
}

/// Representation of a repository.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repository {
    /// Paths to the different workspaces in the repo tree for that repository.
    pub workspaces: Vec<Workspace>,
    /// Identifier of the repository.
    pub id: RepoId,
    /// Type of version control system the repository uses.
    pub vcs: VersionControlSystem,
    /// Boolean indicating if the repository is a git submodule or not.
    pub is_submodule: bool,
    /// File containing the remote information.
    remote_config: TrackedFile,
    /// File the version control system updates when a workspace is added, if
    /// it has one.
    workspace_store: Option<TrackedFile>,
}

impl Repository {
    /// Search for a repository at the given path without printing any warning
    /// about the repository location.
    pub fn discover_silent(
        config: &Config,
        path: &Path,
    ) -> Result<Self, Box<dyn Error>> {
        let mut current_path = Some(path);

        while let Some(root) = current_path {
            match Self::try_new(config, root) {
                Ok(repo) => {
                    return Ok(repo);
                }
                Err(err) => {
                    if err.downcast_ref::<NoRepositoryError>().is_none() {
                        return Err(err);
                    }
                }
            }
            current_path = root.parent();
        }

        Err(Box::new(NoRepositoryError(path.to_path_buf())))
    }

    /// Search for a repository at the given path.
    pub async fn discover(
        ui: &mut Ui<'_>,
        config: &Config,
        path: &Path,
        strategy: ExpectedTreeStrategy,
    ) -> Result<Self, Box<dyn Error>> {
        let repository = Self::discover_silent(config, path)?;
        let workspace = repository.get_latest_workspace();

        if let Some(expected_root) = repository
            .expected_root(ui, config, workspace, strategy)
            .await?
        {
            let root = workspace.path();

            if root != &expected_root && !config.should_be_ignored(root) {
                ui_warning!(
                    ui,
                    "⚠️Unexpected location for the repository {}. Currently \
                     in \"{}\" should be in \"{}\". Run `{}` to fix it.",
                    repository.id.name,
                    root.display(),
                    expected_root.display(),
                    if root.starts_with(&config.root) {
                        "rt clean".to_string()
                    } else {
                        format!("rt insert \"{}\"", root.display())
                    }
                );
            }
        }
        Ok(repository)
    }

    /// Try loading a repository which root is the one provided.
    pub fn try_new(
        config: &Config,
        root: &Path,
    ) -> Result<Repository, Box<dyn Error>> {
        if let Some((vcs, is_submodule)) = VersionControlSystem::try_new(root) {
            let workspace =
                Workspace::new(TreeSpace::from_path(config, root), root);
            let (remote_config, remote_url) =
                vcs.get_repo(&workspace).get_remote_url()?;
            let id = RepoId::from_repo(&root, remote_url.as_ref())?;
            let workspace_store = vcs
                .get_repo(&workspace)
                .get_workspace_store()?
                .map(TrackedFile::new)
                .transpose()?;

            Ok(Self {
                workspaces: Vec::from([workspace]),
                id,
                vcs,
                is_submodule,
                remote_config: TrackedFile::new(remote_config)?,
                workspace_store,
            })
        } else {
            Err(Box::new(NoRepositoryError(root.to_path_buf())))
        }
    }
}

impl Repository {
    /// Find out if the remote configuration of the repository changed, or if
    /// a workspace has been added or removed since the repository was loaded.
    pub fn has_been_modified(&self) -> bool {
        self.remote_config.has_been_modified().unwrap_or(true)
            || self
                .workspace_store
                .as_ref()
                .is_some_and(|f| f.has_been_modified().unwrap_or(true))
            || self.workspaces.iter().any(|w| !w.path().exists())
    }

    /// Add a workspace to the repository.
    pub fn add_workspace(&mut self, other: Self) {
        self.workspaces.extend(other.workspaces);
    }

    /// Find out if the repository has the specified workspace.
    fn has_workspace(&self, workspace: &Workspace) -> bool {
        self.workspaces.iter().any(|w| w == workspace)
    }

    /// Try to get the workspace located in the specified tree-space.
    pub fn get_tree_workspace(
        &self,
        ui: &Ui,
        tree_space: &TreeSpace,
    ) -> Option<&Workspace> {
        let res: Vec<&Workspace> = self
            .workspaces
            .iter()
            .filter(|workspace| workspace.is_tree_workspace(tree_space))
            .collect();

        if res.is_empty() {
            None
        } else {
            if res.len() != 1 {
                ui.warning(
                    "Found several copies of a same repository for a same \
                     tree-space",
                );
            }
            Some(res[0])
        }
    }

    /// Get the workspace which corresponds to the main repository.
    pub fn get_main_workspace(&self, ui: &Ui) -> &Workspace {
        if self.workspaces.len() == 1 {
            return &self.workspaces[0];
        }
        let res: Vec<&Workspace> = self
            .workspaces
            .iter()
            .filter(|workspace| workspace.is_main_workspace())
            .collect();

        if res.len() > 1 {
            ui.warning("Found several copies of a same main repository.");
        }
        // Without main workspace, which only happens with a modified cache,
        // fall back to the first workspace.
        res.first().copied().unwrap_or_else(|| {
            self.workspaces
                .first()
                .expect("a repository always has at least one workspace")
        })
    }

    /// Get the workspace which has been added to the repository last. Should
    /// correspond to the last workspace discovered for that repository.
    pub fn get_latest_workspace(&self) -> &Workspace {
        self.workspaces
            .iter()
            .last()
            .expect("Must have at least 1 element")
    }

    /// Get the expected path to the root of the repository within the repo
    /// tree. If the repository is a submodule then, it has to be at its place
    /// within its main repository and therefore we return None.
    // TODO: Cache the result of that function. This might do API access which
    // we should not have to do uselessly multiple times.
    pub async fn expected_root(
        &self,
        ui: &mut Ui<'_>,
        config: &Config,
        workspace: &Workspace,
        strategy: ExpectedTreeStrategy,
    ) -> Result<Option<PathBuf>, Box<dyn Error>> {
        assert!(self.has_workspace(workspace));
        Ok(if self.is_submodule {
            None
        } else if let Some(tree_space) = workspace.tree_space()
            && tree_space.kind().is_workspace()
        {
            // Additional workspaces belong to their own tree-space.
            Some(tree_space.repo_location(config, &self.id)?)
        } else {
            Some(
                self.id
                    .expected_tree(ui, config, Some(workspace.path()), strategy)
                    .await?
                    .repo_location(config, &self.id)?,
            )
        })
    }

    /// Get the git submodules present in the repository.
    pub fn submodules(
        &self,
        workspace: &Workspace,
    ) -> Result<Vec<SubmoduleInfo>, Box<dyn Error>> {
        assert!(self.has_workspace(workspace));

        Ok(if self.vcs.is_git() {
            git::submodules::get(workspace.path(), &self.id.remote)?
        } else {
            Vec::new()
        })
    }

    /// Get the struct to use to interact with the version control system of the
    /// repository.
    pub fn get_vcs_repo<'workspace>(
        &self,
        workspace: &'workspace Workspace,
    ) -> Box<dyn VcsRepository + 'workspace> {
        assert!(self.has_workspace(workspace));

        self.vcs.get_repo(workspace)
    }

    /// Add a workspace in the specified tree-space.
    pub fn add_workspace_in_tree(
        &self,
        ui: &Ui,
        config: &Config,
        tree_space: &TreeSpace,
    ) -> Result<(), Box<dyn Error>> {
        let workspace_root = tree_space.repo_location(config, &self.id)?;

        self.get_vcs_repo(self.get_main_workspace(ui))
            .add_workspace(tree_space.name(config), &workspace_root)?;

        // TODO mutable RepoTree requires some work...
        // self.workspaces.push(Workspace::new(Some(tree_space.clone()),
        // &workspace_root));

        Ok(())
    }
}

#[cfg(test)]
impl Repository {
    /// Create a repository for tests, without reading anything on disk.
    pub fn for_test(
        id: RepoId,
        vcs: VersionControlSystem,
        workspaces: Vec<Workspace>,
    ) -> Self {
        Self {
            workspaces,
            id,
            vcs,
            is_submodule: false,
            remote_config: TrackedFile {
                file: PathBuf::from("/nonexistent/config"),
                last_modified: Timestamp::UNIX_EPOCH,
            },
            workspace_store: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Configuration whose root is a new temporary directory.
    fn config() -> (tempfile::TempDir, Config) {
        let dir = tempfile::tempdir().expect("cannot create directory");
        let mut config = Config::test_default();
        config.root = dir.path().to_path_buf();
        (dir, config)
    }

    /// Create the directories `paths` in the root of `config`.
    fn create_dirs(config: &Config, paths: &[&str]) {
        for path in paths {
            fs::create_dir_all(config.root.join(path))
                .expect("cannot create directory");
        }
    }

    #[test]
    fn rm_removes_empty_parents_up_to_the_tree_space() {
        let (_dir, config) = config();
        create_dirs(&config, &["agent/test/owner/repo/src", "agent/other"]);
        let workspace = Workspace::Tree(
            TreeSpace::Agent,
            config.root.join("agent/test/owner/repo"),
        );

        workspace.rm(&config).expect("removable workspace");

        assert!(!config.root.join("agent/test").exists());
        assert!(config.root.join("agent/other").is_dir());
    }

    #[test]
    fn rm_keeps_the_tree_space_directory() {
        let (_dir, config) = config();
        create_dirs(&config, &["agent/test/owner/repo"]);
        let workspace = Workspace::Tree(
            TreeSpace::Agent,
            config.root.join("agent/test/owner/repo"),
        );

        workspace.rm(&config).expect("removable workspace");

        assert!(config.root.join("agent").is_dir());
        assert!(!config.root.join("agent/test").exists());
    }

    #[test]
    fn rm_stops_at_the_first_non_empty_parent() {
        let (_dir, config) = config();
        create_dirs(&config, &["dev/test/owner/repo", "dev/test/owner/other"]);
        let workspace = Workspace::Tree(
            TreeSpace::Dev,
            config.root.join("dev/test/owner/repo"),
        );

        workspace.rm(&config).expect("removable workspace");

        assert!(!config.root.join("dev/test/owner/repo").exists());
        assert!(config.root.join("dev/test/owner/other").is_dir());
    }

    #[test]
    fn rm_outside_the_tree_removes_only_the_workspace() {
        let (dir, config) = config();
        let outside = tempfile::tempdir().expect("cannot create directory");
        let path = outside.path().join("parent/repo");
        fs::create_dir_all(&path).expect("cannot create directory");

        Workspace::Other(path.clone())
            .rm(&config)
            .expect("removable");

        assert!(!path.exists());
        assert!(outside.path().join("parent").is_dir());
        assert!(dir.path().is_dir());
    }

    #[test]
    fn rm_of_missing_workspace_fails() {
        let (_dir, config) = config();
        let workspace = Workspace::Tree(
            TreeSpace::Dev,
            config.root.join("dev/test/missing"),
        );

        assert!(workspace.rm(&config).is_err());
    }

    /// Repository with workspaces in `tree_spaces`.
    fn repository(tree_spaces: &[TreeSpace]) -> Repository {
        Repository::for_test(
            RepoId::from_remote_url("https://test.com/o/r").expect("valid URL"),
            VersionControlSystem::Git,
            tree_spaces
                .iter()
                .map(|t| {
                    Workspace::Tree(t.clone(), PathBuf::from(format!("/{t:?}")))
                })
                .collect(),
        )
    }

    #[test]
    fn main_and_tree_workspaces() {
        let config = Config::test_default();
        let ui = Ui::new(&config);
        let repository = repository(&[TreeSpace::Agent, TreeSpace::Dev]);

        assert_eq!(
            repository.get_main_workspace(&ui).tree_space(),
            Some(&TreeSpace::Dev)
        );
        assert_eq!(
            repository
                .get_tree_workspace(&ui, &TreeSpace::Agent)
                .and_then(Workspace::tree_space),
            Some(&TreeSpace::Agent)
        );
        assert!(
            repository
                .get_tree_workspace(&ui, &TreeSpace::Archive)
                .is_none()
        );
        assert_eq!(
            repository.get_latest_workspace().tree_space(),
            Some(&TreeSpace::Dev)
        );
    }

    #[test]
    fn main_workspace_without_main_tree_space_falls_back_to_the_first() {
        let config = Config::test_default();
        let ui = Ui::new(&config);
        let mut repository = repository(&[TreeSpace::Agent]);
        repository
            .workspaces
            .push(Workspace::Tree(TreeSpace::Agent, PathBuf::from("/other")));

        assert_eq!(
            repository.get_main_workspace(&ui).path(),
            &PathBuf::from("/Agent")
        );
    }
}
