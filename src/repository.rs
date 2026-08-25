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
use crate::error::NotImplementedError;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repo_id::RepoId;
use crate::repo_state::RepoState;
use crate::tree::TreeSpace;
use crate::utils::get_last_modified;
use crate::version_control_system::VersionControlSystem;
use crate::version_control_system::git::SubmoduleInfo;
use crate::version_control_system::git::{self};
use crate::version_control_system::jujutsu;

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

/// Representation of a repository.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Repository {
    /// Identifier of the repository.
    pub tree: Option<TreeSpace>,
    /// Identifier of the repository.
    pub id: RepoId,
    /// Type of version control system the repository uses.
    pub vcs: VersionControlSystem,
    /// Boolean indicating if the repository is a git submodule or not.
    pub is_submodule: bool,
    /// Path to the root of the repository.
    pub root: PathBuf,
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
                Ok(repo) => return Ok(repo),
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

        if let Some(expected_root) =
            repository.expected_root(config, strategy).await?
            && repository.root != expected_root
            && !config.should_be_ignored(&repository.root)
        {
            eprintln!(
                "⚠️Unexpected location for the repository {}. Currently in \
                 \"{}\" should be in \"{}\". Run `{}` to fix it.",
                repository.id.name,
                repository.root.display(),
                expected_root.display(),
                if repository.root.starts_with(&config.root) {
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
        root: &Path,
    ) -> Result<Self, Box<dyn Error>> {
        if let Some((vcs, is_submodule)) = VersionControlSystem::try_new(root) {
            let (remote_config, remote_url) = match vcs {
                VersionControlSystem::Git
                | VersionControlSystem::JujutsuGit => {
                    git::get_remote_url(root)?
                }
                VersionControlSystem::Jujutsu => jujutsu::get_remote_url(root)?,
            };
            let id = RepoId::from_repo(&root, remote_url.as_ref())?;

            let tree = TreeSpace::from_path(config, root);

            Ok(Self {
                tree,
                id,
                vcs,
                is_submodule,
                root: root.to_path_buf(),
                remote_config: RemoteConfig::new(remote_config)?,
            })
        } else {
            Err(Box::new(NoRepositoryError(root.to_path_buf())))
        }
    }

    /// Get the expected path to the root of the repository within the repo
    /// tree. If the repository is a submodule then, it has to be at its place
    /// within its main repository and therefore we return None.
    // TODO: Cache the result of that function. This might do API access which
    // we should not have to do uselessly multiple times.
    pub async fn expected_root(
        &self,
        config: &Config,
        strategy: ExpectedTreeStrategy,
    ) -> Result<Option<PathBuf>, Box<dyn Error>> {
        Ok(if self.is_submodule {
            None
        } else {
            Some(
                self.id
                    .expected_tree(config, Some(&self.root), strategy)
                    .await?
                    .repo_location(config, &self.id)?,
            )
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
    pub async fn state(&self) -> Result<RepoState, Box<dyn Error>> {
        Ok(match self.vcs {
            VersionControlSystem::Jujutsu
            | VersionControlSystem::JujutsuGit => {
                jujutsu::get_repo_state(&self.root).await?
            }
            vcs => Err(NotImplementedError(format!(
                "Repository state for {vcs} Version Control"
            )))?,
        })
    }
}
