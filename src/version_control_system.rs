//! Enumeration listing the different type of Version Control System we support.
use std::{
    fmt::Display,
    fs::{canonicalize, read_to_string},
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use colored::Colorize;

use crate::jujutsu;

#[derive(Debug, Copy, Clone, ValueEnum, PartialEq)]
pub enum VersionControlSystem {
    /// git
    Git,
    /// jj
    Jujutsu,
    /// Jujutsu collocated with Git.
    JujutsuGit,
}

impl VersionControlSystem {
    pub fn discover_root(
        path: PathBuf,
    ) -> Option<(PathBuf, Self, bool, bool, Option<PathBuf>)> {
        let mut current_path = Some(path);
        while current_path.is_some() {
            let root = current_path.clone().unwrap();
            if let Some((vcs, is_git_submodule, is_jj_workspace, git_dir)) =
                VersionControlSystem::try_new(&root)
            {
                return current_path.map(|cp| {
                    (cp, vcs, is_git_submodule, is_jj_workspace, git_dir)
                });
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
    pub fn try_new(dir: &Path) -> Option<(Self, bool, bool, Option<PathBuf>)> {
        let jj_dir = dir.join(".jj");
        let is_jj = jj_dir.is_dir();
        let is_jj_workspace = jj_dir.join("repo").is_file();
        let git_dir = dir.join(".git");
        let is_git = git_dir.exists();
        let is_git_submodule = git_dir.is_file();

        let vcs = match (is_jj, is_git) {
            (true, true) => Self::JujutsuGit,
            (true, false) => Self::Jujutsu,
            (false, true) => Self::Git,
            (false, false) => return None,
        };

        let git_dir = if is_jj {
            jujutsu::git::get_git_backend_path(dir).ok()
        } else if is_git {
            if is_git_submodule {
                canonicalize(git_dir.parent().unwrap().join(
                    read_to_string(&git_dir).unwrap_or_else(|_| {
                        panic!("Error reading {}", git_dir.display())
                    }),
                ))
                .ok()
            } else {
                Some(git_dir)
            }
        } else {
            panic!("Should not happen");
        };

        Some((vcs, is_git_submodule, is_jj_workspace, git_dir))
    }

    pub fn is_git(&self) -> bool {
        matches!(self, Self::Git | Self::JujutsuGit)
    }

    pub fn is_jujutsu(&self) -> bool {
        matches!(self, Self::Jujutsu | Self::JujutsuGit)
    }

    pub fn short_display(&self) -> String {
        match self {
            Self::Git => "󰊢".ansi_color(166).to_string(),
            Self::Jujutsu => "".blue().to_string(),
            Self::JujutsuGit => format!(
                "{}{}",
                Self::Git.short_display(),
                Self::Jujutsu.short_display(),
            ),
        }
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
