//! Enumeration listing the different type of Version Control System we support.
use std::{fmt::Display, path::Path};

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
    pub fn try_new(dir: &Path) -> Option<Self> {
        fn has_dir(search_dir: &Path, dir: &str) -> bool {
            let mut search_dir = search_dir.to_path_buf().clone();
            search_dir.push(dir);

            search_dir.exists()
        }

        let is_jj = has_dir(dir, ".jj");
        if has_dir(dir, ".git") {
            if is_jj {
                Some(Self::JujutsuGit)
            } else {
                Some(Self::Git)
            }
        } else if is_jj {
            Some(Self::Jujutsu)
        } else if has_dir(dir, ".hg") {
            Some(Self::Mercurial)
        } else if has_dir(dir, ".svn") {
            Some(Self::Subversion)
        } else {
            None
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
                Self::Subversion => "svn",
                Self::GitSubversion => "git-svn",
                Self::Jujutsu => "jj",
                Self::JujutsuGit => "jj collocated git",
                Self::Mercurial => "hg",
            }
        )
    }
}
