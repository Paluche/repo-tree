//! Enumeration listing the different type of Version Control System we support.
pub mod git;
pub mod jujutsu;
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use clap::ValueEnum;
use serde::Deserialize;
use serde::Serialize;

use crate::config::Config;
use crate::prompt::Prompt;
use crate::repo_state::RepoState;
use crate::repository::Workspace;

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
        // The parent of a relative path ends with an empty path, which is not
        // to be searched as the current directory.
        path.ancestors()
            .take_while(|dir| !dir.as_os_str().is_empty())
            .find_map(|dir| {
                Self::try_new(dir).map(|(vcs, is_submodule)| {
                    (dir.to_path_buf(), vcs, is_submodule)
                })
            })
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

    /// Get the struct to use to interact with a repository using this version
    /// control system.
    pub fn get_repo<'repo_tree>(
        &self,
        workspace: &'repo_tree Workspace,
    ) -> Box<dyn VcsRepository + 'repo_tree> {
        match self {
            Self::Git => Box::new(git::GitVcs::new(workspace)),
            Self::Jujutsu => {
                Box::new(jujutsu::JujutsuVcs::new(workspace, false))
            }
            Self::JujutsuGit => {
                Box::new(jujutsu::JujutsuVcs::new(workspace, true))
            }
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

/// Representation of a Repository workspace.
pub struct VcsWorkspace {
    /// Name of the workspace.
    pub name: String,

    /// Path where the workspace is located.
    pub path: PathBuf,
}

impl VcsWorkspace {
    /// Get a struct that will implement the Display trait to display the
    /// workspace name.
    pub fn display_name<'vcs_workspace, 'config>(
        &'vcs_workspace self,
        config: &'config Config,
    ) -> VcsWorkspaceDisplayName<'vcs_workspace, 'config> {
        VcsWorkspaceDisplayName {
            vcs_workspace: self,
            config,
        }
    }
}

/// Implement the Display trait for VcsWorkspace, displaying the name of the
/// workspace.
pub struct VcsWorkspaceDisplayName<'vcs_workspace, 'config> {
    /// The VcsWorkspace struct to display.
    vcs_workspace: &'vcs_workspace VcsWorkspace,
    /// The configuration dictating how to display.
    config: &'config Config,
}

impl<'vcs_workspace, 'config> Display
    for VcsWorkspaceDisplayName<'vcs_workspace, 'config>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.config
                .prompt
                .workspace
                .colorize(format!("{}@", self.vcs_workspace.name))
        )
    }
}

/// Representation of a repository remote.
pub struct VcsRemote {
    /// Name of the remote.
    pub name: String,
    /// URL of the remote.
    pub url: String,
}

/// Functions to interact with a supported Version Control System.
pub trait VcsRepository {
    /// Get the remote URL of the repository to use to organize the repository
    /// within the repo tree. This would be either the origin remote or the
    /// first defined remote.
    fn get_remote_url(
        &self,
    ) -> Result<(PathBuf, Option<String>), Box<dyn Error>>;

    /// Get the path to the file the version control system updates when a
    /// workspace is added, if there is one.
    fn get_workspace_store(&self) -> Result<Option<PathBuf>, Box<dyn Error>>;

    /// Get the configured remotes of the repository.
    fn get_remotes(&self) -> Result<Vec<VcsRemote>, Box<dyn Error>>;

    /// Clone a repository.
    fn clone(&self, remote_url: &str) -> Result<(), Box<dyn Error>>;

    /// Fetch the repository.
    fn fetch(&self, quiet: bool) -> Result<(), Box<dyn Error>>;

    /// Build the prompt line for a repository.
    fn prompt(
        &self,
        config: &Config,
        prompt: &mut Prompt<'_>,
    ) -> Result<(), Box<dyn Error>>;

    /// Get the repository state.
    fn get_repo_state(&self) -> Result<RepoState, Box<dyn Error>>;

    /// The name of the workspace associated with the repository.
    fn get_workspace(&self) -> Result<Option<VcsWorkspace>, Box<dyn Error>>;

    /// Create a new workspace of the repository.
    fn add_workspace(
        &self,
        name: &str,
        destination: &Path,
    ) -> Result<(), Box<dyn Error>>;

    /// Remove the workspace.
    fn remove_workspace(&self, config: &Config) -> Result<(), Box<dyn Error>>;

    /// Forget the workspaces of the repository which directory does not exist
    /// anymore, and return them.
    fn forget_missing_workspaces(
        &self,
    ) -> Result<Vec<VcsWorkspace>, Box<dyn Error>>;
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_utils::no_colors;

    /// Create a directory containing the given marker directories and files.
    fn repo(dirs: &[&str], files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("cannot create directory");
        for name in dirs {
            fs::create_dir(dir.path().join(name))
                .expect("cannot create directory");
        }
        for name in files {
            fs::write(dir.path().join(name), "gitdir: ../.git/modules/x\n")
                .expect("cannot create file");
        }
        dir
    }

    #[test]
    fn detect_version_control_system() {
        use VersionControlSystem::*;

        for (dirs, files, expected) in [
            (&[".git"][..], &[][..], Some((Git, false))),
            (&[], &[".git"], Some((Git, true))),
            (&[".jj"], &[], Some((Jujutsu, false))),
            (&[".jj", ".git"], &[], Some((JujutsuGit, false))),
            (&[".jj"], &[".git"], Some((JujutsuGit, true))),
            (&[], &[".jj"], None),
            (&["git", "jj"], &[], None),
            (&[], &[], None),
        ] {
            let dir = repo(dirs, files);

            assert_eq!(
                VersionControlSystem::try_new(dir.path()),
                expected,
                "{dirs:?} {files:?}"
            );
        }
    }

    #[test]
    fn discover_root_walks_up_to_the_repository() {
        let dir = repo(&[".git"], &[]);
        let sub = dir.path().join("a/b");
        fs::create_dir_all(&sub).expect("cannot create directory");

        assert_eq!(
            VersionControlSystem::discover_root(sub),
            Some((dir.path().to_path_buf(), VersionControlSystem::Git, false))
        );
    }

    #[test]
    fn discover_root_finds_the_closest_repository() {
        let dir = repo(&[".git"], &[]);
        let sub = dir.path().join("sub");
        fs::create_dir_all(sub.join(".jj")).expect("cannot create directory");

        assert_eq!(
            VersionControlSystem::discover_root(sub.join("x")),
            Some((sub, VersionControlSystem::Jujutsu, false))
        );
    }

    #[test]
    fn discover_root_outside_repository() {
        let dir = repo(&[], &[]);

        // The temporary directory itself is not inside a repository.
        if let Some((root, _, _)) =
            VersionControlSystem::discover_root(dir.path().to_path_buf())
        {
            assert!(!root.starts_with(dir.path()));
        }
        assert_eq!(
            VersionControlSystem::discover_root(PathBuf::from("relative/x")),
            None
        );
    }

    #[test]
    fn version_control_system_names() {
        use VersionControlSystem::*;

        assert_eq!(
            [Git, Jujutsu, JujutsuGit].map(|v| v.to_string()),
            ["git", "jj", "jj-git"]
        );
        assert_eq!(
            [Git, Jujutsu, JujutsuGit].map(|v| {
                toml::Value::try_from(v)
                    .expect("serializable")
                    .as_str()
                    .expect("serialized as a string")
                    .to_string()
            }),
            ["git", "jujutsu", "jujutsu-git"]
        );
        assert_eq!(
            [Git, Jujutsu, JujutsuGit].map(|v| v.is_git()),
            [true, false, true]
        );
        assert_eq!(
            [Git, Jujutsu, JujutsuGit].map(|v| v.is_jujutsu()),
            [false, true, true]
        );
    }

    #[test]
    fn version_control_system_short_display() {
        let _colors = no_colors();
        let config = Config::test_default();
        let git = config.prompt.vcs.git.to_string();
        let jj = config.prompt.vcs.jj.to_string();

        assert_eq!(
            VersionControlSystem::Git.short_display(&config).to_string(),
            git
        );
        assert_eq!(
            VersionControlSystem::Jujutsu
                .short_display(&config)
                .to_string(),
            jj
        );
        assert_eq!(
            VersionControlSystem::JujutsuGit
                .short_display(&config)
                .to_string(),
            format!("{git}{jj}")
        );
    }
}
