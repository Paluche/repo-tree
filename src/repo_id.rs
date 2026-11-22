//! Tools around parsing of repositories URL.
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::sync::LazyLock;

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
use crate::tree_space::TreeSpace;
use crate::ui::Ui;

/// Regular expression of scheme-based URLs, e.g.:
///   `https://github.com/owner/repo.git`
///   `https://oauth2:<token>@github.com/owner/repo.git`
///   `ssh://user@host:2222/owner/repo.git`
///   `git://host/owner/repo`
/// Captures: scheme, user (optional), host, port (optional), path.
static RE_SCHEME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"^(?P<scheme>(?:git|ssh|https?|git\+ssh|rsync))",
        r"://(?:(?P<user>[^@]+)@)?(?P<host>[^/:]+)",
        r":?(?:(?P<port>\d+))?/(~[^/]+/)?(?P<path>[^ \r\n]+?)(?:\.git)?/?$"
    ))
    .expect("hardcoded regular expression is valid")
});

/// Regular expression of scp-like URLs, e.g.:
///   `git@github.com:owner/repo.git`
///   `user@host:/absolute/path/to/repo.git`
/// Captures: user (optional), host, path.
static RE_SCP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:(?P<user>[^@:\s]+)@)?(?P<host>[^:/\s]+):(?P<path>[^ \r\n]+?)(?:\.git)?/?$",
    )
    .expect("hardcoded regular expression is valid")
});

/// Find out if a remote URL designates a repository on the local filesystem
/// (a path or a `file://` URL), which has no host to organize it by.
pub fn is_local_path_url(url: &str) -> bool {
    url.starts_with("file://")
        || url.starts_with('/')
        || url.starts_with("./")
        || url.starts_with("../")
}

/// Parse the remote URL, to capture the different parts.
fn capture_url(url: &str) -> Result<regex::Captures<'_>, ParseUrlError> {
    if is_local_path_url(url) {
        return Err(ParseUrlError(url.to_string()));
    }

    let regex = if url.contains("://") {
        &RE_SCHEME
    } else {
        &RE_SCP
    };
    let captures = regex
        .captures(url)
        .ok_or_else(|| ParseUrlError(url.to_string()))?;

    // The path is used to build the location of the repository in the tree,
    // it must not be able to point outside of it.
    let path = captures["path"].trim_start_matches('/');
    if path
        .split('/')
        .any(|component| matches!(component, "" | "." | ".."))
    {
        return Err(ParseUrlError(url.to_string()));
    }

    Ok(captures)
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
        let name = remote_cap["path"].trim_start_matches('/').to_string();

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
    /// This version (in regard to `from_remote_url()`) defaults to a local
    /// repository named after its directory if the `remote_url` argument is
    /// None or designates a repository on the local filesystem.
    pub fn from_repo<P: AsRef<Path>>(
        repo_path: &P,
        remote_url: Option<&String>,
    ) -> Result<RepoId, ParseUrlError> {
        if let Some(remote_url) = remote_url
            && !is_local_path_url(remote_url)
        {
            Self::from_remote_url(remote_url)
        } else {
            let repo_path = repo_path.as_ref();
            Ok(Self {
                remote: None,
                name: repo_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        ParseUrlError(repo_path.display().to_string())
                    })?
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
                .ok_or(UnknownRemoteHostError(remote.host_url.clone()))
                .map(Some),
            None => Ok(None),
        }
    }

    /// Get the host representation associated with that repository. This method
    /// is deterministic compared to getting the category using
    /// `host_category()`.
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
    /// `host_category()`.
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
        ui: &mut Ui<'_>,
        config: &Config,
        repo_path: Option<&Path>,
        strategy: ExpectedTreeStrategy,
    ) -> Result<TreeSpace, Box<dyn Error>> {
        self.expected_tree_with_forge(
            ui,
            config,
            repo_path,
            strategy,
            self.forge_api(config),
        )
        .await
    }

    /// In which tree the repository should be, based on its ID, using
    /// `forge_api` to interact with the forge hosting the repository.
    async fn expected_tree_with_forge(
        &self,
        ui: &mut Ui<'_>,
        config: &Config,
        repo_path: Option<&Path>,
        strategy: ExpectedTreeStrategy,
        forge_api: Result<Option<Box<dyn ForgeApi>>, Box<dyn Error>>,
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

        match forge_api {
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
                        ui.hint_once(err);
                    }
                    dev_or_archive(config, strategy, repo_path)
                } else {
                    Err(err)
                }
            }
        }
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

    /// Obtain a struct implementing the display for the `RepoId`.
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

/// Struct to display a `RepoId`.
pub struct RepoIdDisplay<'repo_id, 'config> {
    /// `RepoId` to display.
    repo_id: &'repo_id RepoId,
    /// Host category data of the `RepoId`.
    remote_host_name: Option<&'config str>,
}

impl Display for RepoIdDisplay<'_, '_> {
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
    use std::path::PathBuf;

    use pollster::FutureExt;

    use super::*;

    /// Parse `url` into its host and name.
    fn parse(url: &str) -> Result<(String, String), ParseUrlError> {
        let id = RepoId::from_remote_url(url)?;
        let remote = id.remote.expect("a remote URL has a remote");
        assert_eq!(remote.url, url);
        Ok((remote.host_url, id.name))
    }

    #[test]
    fn parse_supported_urls() {
        for (url, host, name) in [
            (
                "https://github.com/owner/repo.git",
                "github.com",
                "owner/repo",
            ),
            ("https://github.com/owner/repo", "github.com", "owner/repo"),
            ("https://github.com/owner/repo/", "github.com", "owner/repo"),
            ("http://host:8080/owner/repo.git", "host", "owner/repo"),
            (
                "https://oauth2:token@gitlab.com/group/sub/repo.git",
                "gitlab.com",
                "group/sub/repo",
            ),
            ("ssh://git@host:2222/owner/repo.git", "host", "owner/repo"),
            ("ssh://host/~user/repo.git", "host", "repo"),
            (
                "git://git.kernel.org/pub/linux.git",
                "git.kernel.org",
                "pub/linux",
            ),
            ("git+ssh://git@host/owner/repo", "host", "owner/repo"),
            ("git@github.com:owner/repo.git", "github.com", "owner/repo"),
            ("github.com:owner/repo", "github.com", "owner/repo"),
            ("user@host:/srv/git/repo.git", "host", "srv/git/repo"),
        ] {
            assert_eq!(
                parse(url).unwrap_or_else(|e| panic!("{url}: {e}")),
                (host.to_string(), name.to_string()),
                "{url}"
            );
        }
    }

    #[test]
    fn parse_unsupported_urls_fails() {
        for url in [
            "",
            "not a url",
            "https://github.com",
            "https://github.com/",
            "file:///srv/git/repo.git",
            "/srv/git/repo.git",
            "./repo",
            "../repo",
        ] {
            assert!(parse(url).is_err(), "{url}");
        }
    }

    #[test]
    fn parse_url_escaping_the_tree_fails() {
        for url in [
            "https://github.com/../../etc",
            "https://github.com/owner/../../repo.git",
            "git@github.com:owner/./repo.git",
            "https://github.com/owner//repo.git",
        ] {
            assert!(parse(url).is_err(), "{url}");
        }
    }

    #[test]
    fn from_repo_without_remote_is_named_after_its_directory() {
        let id = RepoId::from_repo(&PathBuf::from("/tree/local/project"), None)
            .expect("valid directory name");

        assert!(id.is_local());
        assert_eq!(id.name, "project");
    }

    #[test]
    fn from_repo_with_local_path_remote_is_local() {
        for url in ["/srv/git/upstream.git", "file:///srv/git/upstream.git"] {
            let id = RepoId::from_repo(
                &PathBuf::from("/tree/local/project"),
                Some(&url.to_string()),
            )
            .expect("valid directory name");

            assert!(id.is_local(), "{url}");
            assert_eq!(id.name, "project");
        }
    }

    #[test]
    fn from_repo_without_directory_name_fails() {
        assert!(RepoId::from_repo(&PathBuf::from("/"), None).is_err());
        assert!(RepoId::from_repo(&PathBuf::from("/tree/.."), None).is_err());
    }

    #[test]
    fn from_repo_with_non_utf8_directory_name_fails() {
        use std::os::unix::ffi::OsStrExt;

        let path = PathBuf::from(std::ffi::OsStr::from_bytes(b"/tree/\xff"));

        assert!(RepoId::from_repo(&path, None).is_err());
    }

    #[test]
    fn remote_host_of_known_unknown_and_local_repositories() {
        let config = Config::test_default();
        let known = RepoId::from_remote_url("https://test.com/foo/bar.git")
            .expect("valid URL");
        let unknown = RepoId::from_remote_url("https://nowhere.org/foo/bar")
            .expect("valid URL");
        let local = RepoId::from_repo(&PathBuf::from("/x/bar"), None)
            .expect("valid directory name");

        assert_eq!(
            known
                .remote_host(&config)
                .expect("known host")
                .map(|h| h.category.name.as_str()),
            Some("test")
        );
        assert_eq!(known.remote_host_name(&config), Some("test"));
        assert_eq!(
            known.remote_host_repr(&config),
            Some(&ColoredText::new("󰙨", crate::colors::Color::yellow()))
        );

        let error = unknown.remote_host(&config).expect_err("unknown host");
        assert_eq!(
            error.to_string(),
            "Missing host configuration for nowhere.org"
        );
        assert_eq!(unknown.remote_host_name(&config), Some("unknown"));
        assert_eq!(
            unknown.remote_host_repr(&config),
            Some(&config.unknown_host.repr)
        );

        assert!(local.remote_host(&config).expect("local").is_none());
        assert_eq!(local.remote_host_name(&config), None);
        assert_eq!(local.remote_host_repr(&config), None);
    }

    #[test]
    fn display_repository_identifier() {
        let config = Config::test_default();
        let id = RepoId::from_remote_url("https://test.com/foo/bar.git")
            .expect("valid URL");
        let local = RepoId::from_repo(&PathBuf::from("/x/bar"), None)
            .expect("valid directory name");

        assert_eq!(
            id.display(&config).to_string(),
            "test foo/bar https://test.com/foo/bar.git"
        );
        assert_eq!(local.display(&config).to_string(), "bar");
    }

    #[test]
    fn expected_tree_of_unknown_host_fails() {
        let config = Config::test_default();
        let mut ui = Ui::new(&config);
        let id = RepoId::from_remote_url("https://nowhere.org/foo/bar")
            .expect("valid URL");

        assert!(
            id.expected_tree(
                &mut ui,
                &config,
                None,
                ExpectedTreeStrategy::Lazy
            )
            .block_on()
            .is_err()
        );
    }

    #[test]
    fn expected_tree_exact_without_forge_api_hints_and_uses_dev() {
        let config = Config::test_default();
        let mut ui = Ui::new(&config);
        let id = RepoId::from_remote_url("https://github.com/foo/bar")
            .expect("valid URL");

        for _ in 0..2 {
            let tree = id
                .expected_tree(
                    &mut ui,
                    &config,
                    None,
                    ExpectedTreeStrategy::Exact,
                )
                .block_on()
                .expect("forge API fallback");
            assert_eq!(tree, TreeSpace::Dev);
        }
        assert_eq!(ui.printed_messages().len(), 1);
    }

    /// Fake forge API.
    struct FakeForge {
        /// Whether the repositories are archived on the forge.
        archived: bool,
    }

    #[async_trait::async_trait(?Send)]
    impl ForgeApi for FakeForge {
        async fn is_archived(
            &self,
            _repo_id: &RepoId,
        ) -> Result<bool, Box<dyn Error>> {
            Ok(self.archived)
        }

        async fn get_name(
            &self,
            repo_id: &RepoId,
        ) -> Result<String, Box<dyn Error>> {
            Ok(repo_id.name.clone())
        }
    }

    /// Forge API answering for the repository to be archived or not, or no
    /// forge API (`None`).
    fn forge(archived: Option<bool>) -> Option<Box<dyn ForgeApi>> {
        archived.map(|archived| -> Box<dyn ForgeApi> {
            Box::new(FakeForge { archived })
        })
    }

    /// Expected tree of the repository `foo/bar` of the `test.com` host,
    /// located in `tree` (if any), with `strategy`, and a forge API answering
    /// for the repository to be `archived` (if any).
    fn expected_tree(
        tree: Option<&str>,
        strategy: ExpectedTreeStrategy,
        archived: Option<bool>,
    ) -> TreeSpace {
        let config = Config::test_default();
        let mut ui = Ui::new(&config);
        let id = RepoId::from_remote_url("https://test.com/foo/bar.git")
            .expect("URL is a correct one");
        let repo_path =
            tree.map(|tree| config.root.join(tree).join("test/foo/bar"));

        id.expected_tree_with_forge(
            &mut ui,
            &config,
            repo_path.as_deref(),
            strategy,
            Ok(forge(archived)),
        )
        .block_on()
        .expect("the expected tree can be determined")
    }

    #[test]
    fn expected_tree_without_forge_api() {
        use ExpectedTreeStrategy::*;
        use TreeSpace::*;

        for (tree, strategy, expected) in [
            (Some("dev"), Lazy, Dev),
            (Some("dev"), ForceDev, Dev),
            (Some("dev"), ForceArchive, Archive),
            (Some("dev"), Exact, Dev),
            (Some("archive"), Lazy, Archive),
            (Some("archive"), ForceDev, Dev),
            (Some("archive"), ForceArchive, Archive),
            (Some("archive"), Exact, Archive),
            (Some("local"), Lazy, Dev),
            (None, Lazy, Dev),
            (None, ForceArchive, Archive),
        ] {
            assert_eq!(
                expected_tree(tree, strategy, None),
                expected,
                "{tree:?}"
            );
        }
    }

    #[test]
    fn expected_tree_with_forge_api() {
        use ExpectedTreeStrategy::*;
        use TreeSpace::*;

        for (tree, strategy, archived, expected) in [
            // The forge decides, whatever the location.
            (Some("dev"), Exact, true, Archive),
            (Some("dev"), Exact, false, Dev),
            (Some("archive"), Exact, false, Dev),
            (None, Exact, true, Archive),
            (Some("dev"), ForceDev, true, Archive),
            (Some("archive"), ForceArchive, false, Dev),
            // Lazy trusts the current location when there is one.
            (Some("dev"), Lazy, true, Dev),
            (Some("archive"), Lazy, false, Archive),
            (None, Lazy, true, Archive),
        ] {
            assert_eq!(
                expected_tree(tree, strategy, Some(archived)),
                expected,
                "{tree:?} archived: {archived}"
            );
        }
    }

    #[test]
    fn expected_tree_of_local_repository_is_local() {
        let config = Config::test_default();
        let mut ui = Ui::new(&config);
        let repo_path = config.root.join("dev/foo");
        let id = RepoId::from_repo(&repo_path, None)
            .expect("No remote no parse URL error");

        for strategy in [
            ExpectedTreeStrategy::Lazy,
            ExpectedTreeStrategy::ForceDev,
            ExpectedTreeStrategy::ForceArchive,
            ExpectedTreeStrategy::Exact,
        ] {
            let tree = id
                .expected_tree_with_forge(
                    &mut ui,
                    &config,
                    Some(&repo_path),
                    strategy,
                    Ok(forge(Some(true))),
                )
                .block_on()
                .expect("local repository");

            assert_eq!(tree, TreeSpace::Local);
        }
    }

    #[test]
    fn expected_tree_forwards_forge_errors() {
        let config = Config::test_default();
        let mut ui = Ui::new(&config);
        let id = RepoId::from_remote_url("https://test.com/foo/bar.git")
            .expect("URL is a correct one");

        let result = id
            .expected_tree_with_forge(
                &mut ui,
                &config,
                None,
                ExpectedTreeStrategy::Exact,
                Err("forge unreachable".into()),
            )
            .block_on();

        assert_eq!(
            result.expect_err("forge error").to_string(),
            "forge unreachable"
        );
    }
}
