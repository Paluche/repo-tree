//! Tools around parsing of repositories URL.
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

use crate::colors::ColoredText;
use crate::config::Config;
use crate::config::RemoteHost;
use crate::error::ParseUrlError;
use crate::error::UnimplementedForgeApi;
use crate::error::UnknownRemoteHostError;
use crate::forge::ForgeApi;
use crate::tree::TreeSpace;

/// Parse the remote URL, to capture the different parts.
fn capture_url<'b>(url: &'b str) -> Result<regex::Captures<'b>, ParseUrlError> {
    // scheme-based URLs, e.g.:
    //   https://github.com/owner/repo.git
    //   https://oauth2:<token>@github.com/owner/repo.git
    //   ssh://user@host:2222/owner/repo.git
    //   git://host/owner/repo
    //   file:///path/to/repo.git
    // Captures: scheme, user (optional), host, port (optional), path.
    let re_scheme = Regex::new(concat!(
        r"^(?P<scheme>(?:git|ssh|https?|git\+ssh|rsync|file))",
        r"://(?:(?P<user>[^@]+)@)?(?P<host>[^/:]+)",
        r":?(?:(?P<port>\d+))?/(~[^/]+/)?(?P<path>[^ \r\n]+?)(?:\.git)?/?$"
    ))
    .unwrap();

    // scp-like syntax, e.g.:
    //   git@github.com:owner/repo.git
    //   user@host:/absolute/path/to/repo.git
    // Captures: user (optional), host, path.
    let re_scp = Regex::new(
        r"^(?:(?P<user>[^@:\s]+)@)?(?P<host>[^:\s]+):(?P<path>[^ \r\n]+?)(?:\.git)?/?$"
    ).unwrap();

    re_scheme
        .captures(url)
        .or(re_scp.captures(url))
        .ok_or(ParseUrlError(url.to_string()))
}

/// Representation of the URL of a remote.
#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize)]
pub struct Remote {
    /// URL of the remote.
    pub url: String,

    /// The host part of the URL of the remote.
    pub host_url: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize)]
/// Repository Identifier.
pub struct RepoId {
    /// Information about the host associated with the repository.
    pub remote: Option<Remote>,
    /// Name of the repository.
    pub name: String,
}

/// Definition of the different strategies available to obtain the expected
/// tree-space the repository should be in.
pub enum ExpectedTreeStrategy {
    /// Lazy strategy. Do what you can with what you have access locally. In
    /// other words if if we hesitate somehow between dev and archive
    /// tree-spaces, trust the current repository location.
    Lazy,
    /// Force the tree space to be dev if it cannot be determined based on
    /// the repository ID. In other words, if we hesitate somehow between dev
    /// and archive tree-spaces, choose dev.
    ForceDev,
    /// Force the tree space to be archive if it cannot be determined based on
    /// the repository ID. In other words, if we hesitate somehow between Dev
    /// and Archive tree-space, choose Archive.
    ForceArchive,
    /// We want the exact tree-space, so it might mean to do some API requests
    /// to determinate if the repository is archived.
    Exact,
}

impl RepoId {
    /// Create a new repository ID for a Git repository with the provided remote
    /// URL.
    pub fn from_remote_url(remote_url: &str) -> Result<RepoId, ParseUrlError> {
        let remote_cap = capture_url(remote_url)?;
        let host_url = &remote_cap["host"];
        let name = remote_cap["path"].to_string();

        Ok(Self {
            remote: Some(Remote {
                url: remote_url.to_string(),
                host_url: host_url.to_string(),
            }),
            name,
        })
    }

    /// Parse the provided repository remote URL into a host (as Remote struct)
    /// and the local path the repository should be located at in the repo
    /// tree based according to the URL.
    /// This version (in regard to parse_url()) defaults to the local host
    /// location configuration if the remote_url argument is None.
    pub fn from_repo<P: AsRef<Path>>(
        repo_path: &P,
        remote_url: Option<&String>,
    ) -> Result<RepoId, ParseUrlError> {
        if let Some(remote_url) = remote_url {
            Self::from_remote_url(remote_url)
        } else {
            Ok(Self {
                remote: None,
                name: repo_path
                    .as_ref()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            })
        }
    }

    /// Get the host tree category associated with the repository.
    pub fn remote_host<'config>(
        &self,
        config: &'config Config,
    ) -> Result<Option<&'config RemoteHost>, UnknownRemoteHostError> {
        match &self.remote {
            Some(remote) => config
                .get_remote_host(&remote.host_url)
                .ok_or(UnknownRemoteHostError(remote.host_url.to_string()))
                .map(Some),
            None => Ok(None),
        }
    }

    /// Get the host representation associated with that repository. This method
    /// is deterministic compared to getting the category using
    /// host_category().
    pub fn remote_host_repr<'config>(
        &self,
        config: &'config Config,
    ) -> Option<&'config ColoredText> {
        self.remote_host(config)
            .map_or(Some(&config.unknown_host.repr), |r| {
                r.map(|r| &r.category.repr)
            })
    }

    /// Get the host name associated with that repository. This method is
    /// deterministic compared to getting the category using
    /// host_category().
    pub fn remote_host_name<'config>(
        &self,
        config: &'config Config,
    ) -> Option<&'config str> {
        self.remote_host(config)
            .map_or(Some("unknown"), |c| c.map(|c| c.category.name.as_str()))
    }

    /// In which tree the repository should be, based on its ID.
    pub async fn expected_tree(
        &self,
        config: &Config,
        repo_path: Option<&Path>,
        strategy: ExpectedTreeStrategy,
    ) -> Result<TreeSpace, Box<dyn Error>> {
        if self.remote.is_none() {
            return Ok(TreeSpace::Local);
        }

        fn dev_or_archive(
            config: &Config,
            strategy: ExpectedTreeStrategy,
            repo_path: Option<&Path>,
        ) -> Result<TreeSpace, Box<dyn Error>> {
            Ok(if matches!(strategy, ExpectedTreeStrategy::ForceDev) {
                TreeSpace::Dev
            } else if matches!(strategy, ExpectedTreeStrategy::ForceArchive) {
                TreeSpace::Archive
            } else if let Some(repo_path) = repo_path {
                match TreeSpace::from_path(config, repo_path) {
                    Some(TreeSpace::Archive) => TreeSpace::Archive,
                    None | Some(_) => TreeSpace::Dev,
                }
            } else {
                TreeSpace::Dev
            })
        }

        match self.forge_api(config) {
            Ok(Some(forge_api)) => {
                Ok(
                    // This is case where you might opt for the lazy approach.
                    if matches!(strategy, ExpectedTreeStrategy::Lazy)
                        && let Some(repo_path) = repo_path
                        && let Some(tree_space) =
                            TreeSpace::from_path(config, repo_path)
                    {
                        tree_space
                    } else if forge_api.is_archived(self).await? {
                        TreeSpace::Archive
                    } else {
                        TreeSpace::Dev
                    },
                )
            }
            Ok(None) => dev_or_archive(config, strategy, repo_path),
            Err(err) => {
                if let Some(err) = err.downcast_ref::<UnimplementedForgeApi>() {
                    if matches!(strategy, ExpectedTreeStrategy::Exact) {
                        eprintln!("{err}");
                    }
                    dev_or_archive(config, strategy, repo_path)
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Get the expected path to the root of the repository within the repo
    /// tree. If the repository is a submodule then, it has to be at its place
    /// within its main repository and therefore we return None.
    pub async fn expected_root(
        &self,
        config: &Config,
        repo_path: Option<&Path>,
        strategy: ExpectedTreeStrategy,
    ) -> Result<PathBuf, Box<dyn Error>> {
        Ok(self
            .expected_tree(config, repo_path, strategy)
            .await?
            .repo_location(config, self)?)
    }

    /// Find out if a repository exists only locally (no remote configured).
    pub fn is_local(&self) -> bool {
        self.remote.is_none()
    }

    /// Get the struct to use to interact with the forge where the remote
    /// repository is hosted.
    pub fn forge_api(
        &self,
        config: &Config,
    ) -> Result<Option<Box<dyn ForgeApi>>, Box<dyn Error>> {
        if let Some(remote) = &self.remote {
            if let Some(remote_host) = config.get_remote_host(&remote.host_url)
            {
                if let Some(forge) = &remote_host.info.forge {
                    Ok(Some(forge.api()?))
                } else {
                    Ok(None)
                }
            } else {
                Err(Box::new(UnknownRemoteHostError(remote.host_url.clone())))
            }
        } else {
            Ok(None)
        }
    }

    /// Get the RepoId corresponding to the repository as it is on the forge.
    pub async fn get_forge_id(
        &self,
        config: &Config,
    ) -> Result<Option<Self>, Box<dyn Error>> {
        if let Some(remote_host) = self.remote_host(config)?
            && let Some(forge) = &remote_host.info.forge
        {
            Ok(Some(Self {
                remote: self.remote.clone(),
                name: forge.api()?.get_name(self).await?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Obtain a struct implementing the display for the RepoId.
    pub fn display<'repo_id, 'config>(
        &'repo_id self,
        config: &'config Config,
    ) -> RepoIdDisplay<'repo_id, 'config> {
        RepoIdDisplay {
            repo_id: self,
            remote_host_name: self.remote_host_name(config),
        }
    }
}

/// Struct to display a RepoId.
pub struct RepoIdDisplay<'repo_id, 'config> {
    /// RepoId to display.
    repo_id: &'repo_id RepoId,
    /// Host category data of the RepoId.
    remote_host_name: Option<&'config str>,
}

impl<'repo_id, 'config> Display for RepoIdDisplay<'repo_id, 'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.remote_host_name {
            write!(f, "{name} ")?;
        }
        write!(f, "{}", self.repo_id.name)?;
        if let Some(remote) = &self.repo_id.remote {
            write!(f, " {}", remote.url)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pollster::FutureExt;

    use super::*;

    /// Check the different tree strategies against a repository stored in dev.
    fn check_expected_tree_dev(
        strategy: ExpectedTreeStrategy,
        expected_tree: TreeSpace,
    ) {
        let config = Config::test_default();
        let id = RepoId::from_remote_url("https://test.com/foo/bar.git")
            .expect("URL is a correct one");
        let repo_path = config
            .root
            .join(config.tree.dev.category.dir_name())
            .join("test")
            .join("foo")
            .join("bar");
        let tree = id
            .expected_tree(&config, Some(&repo_path), strategy)
            .block_on()
            .unwrap();

        assert_eq!(tree, expected_tree)
    }

    #[test]
    fn check_expected_tree_dev_lazy() {
        check_expected_tree_dev(ExpectedTreeStrategy::Lazy, TreeSpace::Dev)
    }

    #[test]
    fn check_expected_tree_dev_force_dev() {
        check_expected_tree_dev(ExpectedTreeStrategy::ForceDev, TreeSpace::Dev)
    }

    #[test]
    fn check_expected_tree_dev_force_archive() {
        check_expected_tree_dev(
            ExpectedTreeStrategy::ForceArchive,
            TreeSpace::Archive,
        )
    }

    // XXX Need some API mocking which returns the repository to be not an
    // archive. #[test]
    // fn check_expected_tree_dev_exact_dev() {
    //     check_expected_tree_dev(ExpectedTreeStrategy::Exact, TreeSpace::Dev)
    // }

    // XXX Need some API mocking which returns the repository to be an archive.
    // #[test]
    // fn check_expected_tree_dev_exact_dev() {
    //     check_expected_tree_dev(ExpectedTreeStrategy::Exact,
    // TreeSpace::Archive) }

    /// Check the different tree strategies against a repository stored in
    /// archive.
    fn check_expected_tree_archive(
        strategy: ExpectedTreeStrategy,
        expected_tree: TreeSpace,
    ) {
        let config = Config::test_default();
        let id = RepoId::from_remote_url("https://test.com/foo/bar.git")
            .expect("URL is a correct one");
        let repo_path = config
            .root
            .join(config.tree.archive.category.dir_name())
            .join("test")
            .join("foo")
            .join("bar");
        let tree = id
            .expected_tree(&config, Some(&repo_path), strategy)
            .block_on()
            .unwrap();

        assert_eq!(tree, expected_tree)
    }

    #[test]
    fn check_expected_tree_archive_lazy() {
        check_expected_tree_archive(
            ExpectedTreeStrategy::Lazy,
            TreeSpace::Archive,
        )
    }

    #[test]
    fn check_expected_tree_archive_force_dev() {
        check_expected_tree_archive(
            ExpectedTreeStrategy::ForceDev,
            TreeSpace::Dev,
        )
    }

    #[test]
    fn check_expected_tree_archive_force_archive() {
        check_expected_tree_archive(
            ExpectedTreeStrategy::ForceArchive,
            TreeSpace::Archive,
        )
    }

    // XXX Need some API mocking which returns the repository to be not an
    // archive. #[test]
    // fn check_expected_tree_archive_exact_dev() {
    //     check_expected_tree_archive(ExpectedTreeStrategy::Exact,
    // TreeSpace::Dev) }

    // XXX Need some API mocking which returns the repository to be an archive.
    // #[test]
    // fn check_expected_tree_archive_exact_dev() {
    //     check_expected_tree_archive(ExpectedTreeStrategy::Exact,
    // TreeSpace::Archive) }

    /// No matter the strategy a local repository, without remote will be in the
    /// local tree-space.
    fn check_expected_tree_local(strategy: ExpectedTreeStrategy) {
        let config = Config::default();
        let repo_path = config
            .root
            .join(config.tree.local.category.dir_name())
            .join("foo");
        let id = RepoId::from_repo(&repo_path, None)
            .expect("No remote no parse URL error");
        let tree = id
            .expected_tree(&config, Some(&repo_path), strategy)
            .block_on()
            .unwrap();

        assert_eq!(tree, TreeSpace::Local)
    }

    #[test]
    fn check_expected_tree_lazy_local() {
        check_expected_tree_local(ExpectedTreeStrategy::Lazy);
    }

    #[test]
    fn check_expected_tree_lazy_force_dev() {
        check_expected_tree_local(ExpectedTreeStrategy::ForceDev);
    }

    #[test]
    fn check_expected_tree_lazy_force_archive() {
        check_expected_tree_local(ExpectedTreeStrategy::ForceArchive);
    }

    // XXX Need some API mocking which returns the repository to be not an
    // archive. #[test]
    // fn check_expected_tree_local_exact_dev() {
    //     check_expected_tree_local(ExpectedTreeStrategy::Exact)
    // }

    // XXX Need some API mocking which returns the repository to be an archive.
    // #[test]
    // fn check_expected_tree_local_exact_dev() {
    //     check_expected_tree_local(ExpectedTreeStrategy::Exact)
    // }
}
