//! Representation of a repository.
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::error::NoRepositoryError;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repo_id::RepoId;
use crate::tree_space::TreeSpace;
use crate::tree_space::TreeSpaceKind;
use crate::utils::get_last_modified;
use crate::version_control_system::VcsRepository;
use crate::version_control_system::VersionControlSystem;
use crate::version_control_system::git::SubmoduleInfo;
use crate::version_control_system::git::{self};

/// Metadata about the file containing the repository remote(s).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteConfig {
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
    pub fn has_been_modified(&self) -> Result<bool, Box<dyn Error>> {
        Ok(self.last_modified != get_last_modified(&self.file)?)
    }
}

/// A repository workspace.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
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
}

/// Representation of a repository.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Repository {
    /// Paths to the different workspaces in the repo tree for that repository.
    pub workspaces: Vec<Workspace>,
    /// Identifier of the repository.
    pub id: RepoId,
    /// Type of version control system the repository uses.
    pub vcs: VersionControlSystem,
    /// Boolean indicating if the repository is a git submodule or not.
    pub is_submodule: bool,
    /// Path to the file containing the remote information.
    pub remote_config: RemoteConfig,
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
        config: &Config,
        path: &Path,
        strategy: ExpectedTreeStrategy,
    ) -> Result<Self, Box<dyn Error>> {
        let repository = Self::discover_silent(config, path)?;
        let workspace = repository.get_latest_workspace();

        if let Some(expected_root) = repository
            .expected_root(workspace, config, strategy)
            .await?
        {
            let root = workspace.path();

            if root != &expected_root && !config.should_be_ignored(root) {
                eprintln!(
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
            let (remote_config, remote_url) =
                vcs.get_repo(root).get_remote_url()?;
            let id = RepoId::from_repo(&root, remote_url.as_ref())?;

            let workspace =
                Workspace::new(TreeSpace::from_path(config, root), root);

            Ok(Self {
                workspaces: Vec::from([workspace]),
                id,
                vcs,
                is_submodule,
                remote_config: RemoteConfig::new(remote_config)?,
            })
        } else {
            Err(Box::new(NoRepositoryError(root.to_path_buf())))
        }
    }
}

impl Repository {
    /// Find out if the repository has the specified workspace.
    fn has_workspace(&self, workspace: &Workspace) -> bool {
        self.workspaces.iter().find(|w| w == &workspace).is_some()
    }

    /// Try to get the workspace located in the specified tree-space.
    pub fn get_tree_workspace(
        &self,
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
                eprintln!(
                    "Found several copies of a same repository for a same \
                     tree-space"
                );
            }
            Some(res[0])
        }
    }

    /// Get the workspace which corresponds to the main repository.
    pub fn get_main_workspace(&self) -> &Workspace {
        if self.workspaces.len() == 1 {
            return &self.workspaces[0];
        }
        let res: Vec<&Workspace> = self
            .workspaces
            .iter()
            .filter(|workspace| workspace.is_main_workspace())
            .collect();

        if res.is_empty() {
            panic!();
        } else {
            if res.len() != 1 {
                eprintln!("Found several copies of a same main repository.");
            }
            res[0]
        }
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
        workspace: &Workspace,
        config: &Config,
        strategy: ExpectedTreeStrategy,
    ) -> Result<Option<PathBuf>, Box<dyn Error>> {
        assert!(self.has_workspace(workspace));
        Ok(if self.is_submodule {
            None
        } else {
            let id = if matches!(strategy, ExpectedTreeStrategy::Exact) {
                self.id.get_forge_id(config).await?
            } else {
                None
            }
            .unwrap_or(self.id.clone()); // FIXME clone used.
            Some(
                id.expected_root(
                    config,
                    Some(&self.root(tree_space)?),
                    strategy,
                )
                .await?,
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
    pub fn get_vcs_repo(
        &self,
        workspace: &Workspace,
    ) -> Box<dyn VcsRepository> {
        assert!(self.has_workspace(workspace));

        self.vcs.get_repo(workspace.path())
    }
}
