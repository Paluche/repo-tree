//! Definition of tree spaces.
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use clap::ValueEnum;
use clap::builder::StyledStr;
use clap_complete::engine::ArgValueCompleter;
use clap_complete::engine::CompletionCandidate;
use enum_docs_derive::EnumDocs;
use serde::Deserialize;
use serde::Serialize;
use strum::EnumIter;
use strum::IntoEnumIterator;

use crate::colors::ColoredText;
use crate::config::Config;
use crate::config::TreeCategory;
use crate::error::UnexpectedTreeSpaceError;
use crate::error::UnknownRemoteHostError;
use crate::repo_id::RepoId;

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
}

/// The different kind of tree-space.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Default,
)]
pub enum TreeSpaceKind {
    /// Tree-space containing main repositories.
    #[default]
    Main,
    /// Tree-space containing workspace repositories associated with a main
    /// repository from the Main tree space type.
    Workspace,
}

impl TreeSpaceKind {
    /// Is the tree-space of the workspace kind.
    pub fn is_workspace(&self) -> bool {
        matches!(self, Self::Workspace)
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
    EnumDocs,
    EnumIter,
)]
pub enum TreeSpace {
    /// Main tree, where active, user-modified repository are
    Dev,
    /// Where archived / read-only repositories are stored.
    Archive,
    /// Tree containing repositories workspaces where agents, which brings
    /// modification to your repositories, evolves.
    Agent,
    /// Tree for repositories which exists only locally.
    Local,
}

impl TreeSpace {
    /// Obtain the TreeSpace value based on its name, as defined by the
    /// configuration.
    pub fn from_name(config: &Config, name: &str) -> Option<Self> {
        Self::iter().find(|tree_space| name == tree_space.name(config))
    }

    /// Obtain the TreeSpace value based on a directory name, directory should
    /// match a tree-category.
    pub fn from_dir_name(config: &Config, dir_name: &OsStr) -> Option<Self> {
        Self::iter().find(|tree_space| {
            dir_name == tree_space.category(config).dir_name()
        })
    }

    /// Obtain the TreeSpace value based on a path which should be inside a
    /// tree-space.
    pub fn from_path(config: &Config, path: &Path) -> Option<Self> {
        Self::from_dir_name(
            config,
            path.strip_prefix(&config.root).ok()?.iter().next()?,
        )
    }

    /// Obtain the TreeSpaceKind associated with the tree-space.
    pub fn kind(&self) -> TreeSpaceKind {
        match self {
            Self::Dev | Self::Local | Self::Archive => TreeSpaceKind::Main,
            Self::Agent => TreeSpaceKind::Workspace,
        }
    }

    /// Get the tree category associated with the tree space.
    pub fn category<'config>(
        &self,
        config: &'config Config,
    ) -> &'config TreeCategory {
        match self {
            Self::Dev => &config.tree.dev.category,
            Self::Local => &config.tree.local.category,
            Self::Agent => &config.tree.agent.category,
            Self::Archive => &config.tree.archive.category,
        }
    }

    /// Get the organization model associated with the tree-space.
    fn organization<'config>(
        &self,
        config: &'config Config,
    ) -> TreeOrganization<'config> {
        let category = self.category(config);
        match self {
            Self::Dev => TreeOrganization::RemoteBased(config, category),
            Self::Local => TreeOrganization::Local(config, category),
            Self::Agent => TreeOrganization::RemoteBased(config, category),
            Self::Archive => TreeOrganization::RemoteBased(config, category),
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
        &self.category(config).repr
    }

    /// Get a struct which knows how to display the tree space.
    pub fn display<'tree_space, 'config>(
        &'tree_space self,
        config: &'config Config,
    ) -> TreeSpaceDisplay<'tree_space, 'config> {
        TreeSpaceDisplay {
            tree_space: self,
            config,
        }
    }

    /// Get the name associated with the tree-space.
    pub fn name<'config>(&self, config: &'config Config) -> &'config String {
        &self.category(config).name
    }

    /// Turn the tree-space into a CLI completion candidate.
    fn into_completion_candidate(
        self,
        config: &Config,
        current: &OsStr,
    ) -> Option<CompletionCandidate> {
        let name = self.category(config).name.clone();
        name.starts_with(current.to_str().unwrap_or("")).then_some(
            CompletionCandidate::new(name)
                .help(Some(StyledStr::from(self.doc()))),
        )
    }

    /// CLI completion candidates for a tree space argument.
    pub fn completer() -> ArgValueCompleter {
        ArgValueCompleter::new(|current: &OsStr| {
            Config::load().map_or(Vec::new(), |config| {
                TreeSpace::iter()
                    .filter_map(|tree_space| {
                        tree_space.into_completion_candidate(&config, current)
                    })
                    .collect()
            })
        })
    }
}

/// Struct which knows how to display a TreeSpace.
pub struct TreeSpaceDisplay<'tree_space, 'config> {
    /// TreeSpace instance to display.
    tree_space: &'tree_space TreeSpace,
    /// User configuration which dictates how to display the tree space.
    config: &'config Config,
}

impl<'tree_space, 'config> Display for TreeSpaceDisplay<'tree_space, 'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tree_space.category(self.config).name)
    }
}

/// Enum for CLI arguments where you can specify a workspace tree-space.
#[derive(Debug, Clone, ValueEnum, EnumIter)]
pub enum WorkspaceTreeSpace {
    Agent,
}

impl WorkspaceTreeSpace {
    /// CLI completion candidates for a workspace tree space argument.
    pub fn completer() -> ArgValueCompleter {
        ArgValueCompleter::new(|current: &OsStr| {
            Config::load().map_or(Vec::new(), |config| {
                WorkspaceTreeSpace::iter()
                    .filter_map(|wts| {
                        let tree_space: TreeSpace = wts.into();
                        if matches!(tree_space.kind(), TreeSpaceKind::Workspace)
                        {
                            tree_space
                                .into_completion_candidate(&config, current)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<CompletionCandidate>>()
            })
        })
    }
}

impl From<WorkspaceTreeSpace> for TreeSpace {
    fn from(value: WorkspaceTreeSpace) -> Self {
        match value {
            WorkspaceTreeSpace::Agent => Self::Agent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_tree_space_from_path(
        config: &Config,
        category: &TreeCategory,
        expected_tree_space: Option<TreeSpace>,
    ) {
        let repo_path = config
            .root
            .join(category.dir_name())
            .join("test")
            .join("foo")
            .join("bar");

        let tree_space = TreeSpace::from_path(config, &repo_path);
        assert_eq!(tree_space, expected_tree_space);
    }

    #[test]
    fn check_tree_space_from_path_dev() {
        let config = Config::test_default();

        check_tree_space_from_path(
            &config,
            &config.tree.dev.category,
            Some(TreeSpace::Dev),
        )
    }

    #[test]
    fn check_tree_space_from_path_archive() {
        let config = Config::test_default();

        check_tree_space_from_path(
            &config,
            &config.tree.archive.category,
            Some(TreeSpace::Archive),
        )
    }

    #[test]
    fn check_tree_space_from_path_local() {
        let config = Config::test_default();

        check_tree_space_from_path(
            &config,
            &config.tree.local.category,
            Some(TreeSpace::Local),
        )
    }

    #[test]
    fn check_tree_space_from_path_invalid() {
        let config = Config::test_default();
        let repo_path = PathBuf::from("/home/not-user/work/dev/test/foo/bar");
        let tree_space = TreeSpace::from_path(&config, &repo_path);
        assert_eq!(tree_space, None)
    }
}
