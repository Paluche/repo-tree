//! Enumeration listing the different type of Version Control System we support.
use std::{fmt::Display, path::Path};

#[derive(Debug, Copy, Clone)]
pub enum VersionControlSystem {
    /// git
    Git,
    /// svn
    Subversion,
    /// git-svn
    GitSubversion,
    /// jj
    Jujutsu,
    /// Jujutsu collocated with Git.
    JujutsuGit,
    /// hg
    Mercurial,
}

impl VersionControlSystem {
    /// Try to load the current version control system used by the current repository.
    /// The path must correspond to the root of the repository.
    /// Return a new instance of VersionControlSystem and a boolean to indicate if the repository
    /// is a submodule or not.
    pub fn try_new(dir: &Path) -> Option<(Self, bool)> {
        fn exists(search_dir: &Path, dir: &str) -> (bool, bool) {
            let mut search_dir = search_dir.to_path_buf().clone();
            search_dir.push(dir);

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
        } else if exists(dir, ".hg").0 {
            Some((Self::Mercurial, false))
        } else if exists(dir, ".svn").0 {
            // XXX Subversion can have sub-modules.
            Some((Self::Subversion, false))
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
}

impl Display for VersionControlSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Git => "git",
                Self::Subversion => "svn",
                Self::GitSubversion => "git-svn",
                Self::Jujutsu => "jj",
                Self::JujutsuGit => "jj collocated git",
                Self::Mercurial => "hg",
            }
        )
    }
}
