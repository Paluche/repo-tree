//! Enumeration listing the different type of Version Control System we support.
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use colored::Colorize;

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
            // is_git
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
