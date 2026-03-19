//! Enumeration listing the different type of Version Control System we support.
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use clap::ValueEnum;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;

#[derive(
    Debug, Copy, Clone, PartialEq, Default, ValueEnum, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
/// Representation of the different types of version control system supported.
pub enum VersionControlSystem {
    /// Git.
    Git,
    /// Jujutsu (jj).
    Jujutsu,
    /// Jujutsu collocated with Git.
    #[default]
    JujutsuGit,
}

impl VersionControlSystem {
    /// Discover if there is a version control system at the given path, and
    /// which one exactly.
    /// Returns the path to the root of the repository, the type of version
    /// control system it is, and if the repository is a git submodule or
    /// not.
    pub fn discover_root(path: PathBuf) -> Option<(PathBuf, Self, bool)> {
        let mut current_path = Some(path);
        while current_path.is_some() {
            let root = current_path.clone().unwrap();
            if let Some((vcs, is_submodule)) =
                VersionControlSystem::try_new(&root)
            {
                return current_path.map(|cp| (cp, vcs, is_submodule));
            }
            current_path = current_path
                .and_then(|cp| cp.parent().map(|p| p.to_path_buf()));
        }
        None
    }

    /// Try to load the current version control system used by the current
    /// repository. The path must correspond to the root of the repository.
    /// Return a new instance of VersionControlSystem and a boolean to indicate
    /// if the repository is a submodule or not.
    pub fn try_new(dir: &Path) -> Option<(Self, bool)> {
        fn exists(search_dir: &Path, dir: &str) -> (bool, bool) {
            let search_dir = search_dir.join(dir);

            (search_dir.is_dir(), search_dir.is_file())
        }

        let is_jj = exists(dir, ".jj").0;
        let (is_git_main, is_git_submodule) = exists(dir, ".git");

        if is_git_main || is_git_submodule {
            if is_jj {
                Some((Self::JujutsuGit, is_git_submodule))
            } else {
                Some((Self::Git, is_git_submodule))
            }
        } else if is_jj {
            Some((Self::Jujutsu, false))
        } else {
            None
        }
    }

    /// Find out if the version control system is a Git repository.
    pub fn is_git(&self) -> bool {
        matches!(self, Self::Git | Self::JujutsuGit)
    }

    /// Find out if the version control system is a Jujutsu repository.
    pub fn is_jujutsu(&self) -> bool {
        matches!(self, Self::Jujutsu | Self::JujutsuGit)
    }

    /// Obtain a string giving a short human readable description of the version
    /// control system.
    pub fn short_display<'vcs, 'config>(
        &'vcs self,
        config: &'config Config,
    ) -> ShortDisplay<'vcs, 'config> {
        ShortDisplay { vcs: self, config }
    }
}

impl Display for VersionControlSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Git => "git",
                Self::Jujutsu => "jj",
                Self::JujutsuGit => "jj-git",
            }
        )
    }
}

/// Struct providing a display for a short representation of the
/// VersionControlSystem struct.
pub struct ShortDisplay<'vcs, 'config> {
    /// VersionControlSystem to display as a short representation.
    vcs: &'vcs VersionControlSystem,
    /// Configuration of the rt tool.
    config: &'config Config,
}

impl<'vcs, 'config> Display for ShortDisplay<'vcs, 'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.vcs {
            VersionControlSystem::Git => {
                write!(f, "{}", self.config.prompt.vcs.git)
            }
            VersionControlSystem::Jujutsu => {
                write!(f, "{}", self.config.prompt.vcs.jj)
            }
            VersionControlSystem::JujutsuGit => {
                write!(
                    f,
                    "{}{}",
                    self.config.prompt.vcs.git, self.config.prompt.vcs.jj
                )
            }
        }
    }
}
