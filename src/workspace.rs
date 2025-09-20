use std::{
    env,
    fmt::Display,
    path::{Path, PathBuf},
};

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
    fn try_new(dir: &Path) -> Option<Self> {
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

pub struct Repository {
    vcs: VersionControlSystem,
    root: PathBuf,
}

impl Repository {
    fn try_new(root: PathBuf) -> Option<Self> {
        let vcs = VersionControlSystem::try_new(&root)?;

        Some(Self { vcs, root })
    }

    pub fn vcs(&self) -> &VersionControlSystem {
        &self.vcs
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }
}

impl Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} repository at {}", self.vcs, self.root.display())
    }
}

fn search_repositories(dir: &Path) -> Vec<Repository> {
    let mut ret = Vec::new();
    if !dir.is_dir() {
        return ret;
    }

    for entry in dir.read_dir().expect("read dir call failed").flatten() {
        let root = entry.path();
        if let Some(repo) = Repository::try_new(root.clone()) {
            ret.push(repo);
        } else {
            ret.extend(search_repositories(&root));
        }
    }

    ret
}

pub fn load_workspace() -> Vec<Repository> {
    let work_dir =
        env::var("WORK_DIR").expect("Missing WORK_DIR environment variable");
    let work_dir = Path::new(&work_dir);
    search_repositories(work_dir)
}
