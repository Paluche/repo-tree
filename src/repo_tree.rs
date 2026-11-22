//! Definition of a repository tree. A repository tree is an remote-based
//! organized storage of your repositories. Each tree,
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::fs::create_dir_all;
use std::fs::read_to_string;
use std::fs::rename;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;

use globset::Glob;
use serde::Deserialize;
use serde::Serialize;
use strum::IntoEnumIterator;

use crate::config::Config;
use crate::config::config_file;
use crate::error::NoCacheError;
use crate::error::NoRepositoryError;
use crate::repository::Repository;
use crate::repository::TrackedFile;
use crate::repository::Workspace;
use crate::tree_space::TreeSpace;
use crate::tree_space::TreeSpaceKind;
use crate::ui::Ui;
use crate::ui_error;
use crate::ui_hint;
use crate::ui_warning;

/// Search recursively repositories in a directory.
fn search_dir(
    ui: &Ui<'_>,
    config: &Config,
    tree_space_kind: &TreeSpaceKind,
    repositories: &mut Vec<Repository>,
    empty_dirs: &mut Vec<PathBuf>,
    dir: &Path,
) {
    if !dir.is_dir() {
        return;
    }

    let entries = match dir.read_dir() {
        Ok(entries) => entries,
        Err(err) => {
            ui_error!(
                ui,
                "Unable to read the directory {}: {err}",
                dir.display()
            );
            return;
        }
    };

    let mut empty_dir = true;

    for entry in entries.flatten() {
        empty_dir = false;
        let root = entry.path();

        match Repository::try_new(config, &root) {
            Ok(repo) => {
                if !tree_space_kind.is_workspace() {
                    repositories.push(repo);
                } else if let Some(main_repo) =
                    repositories.iter_mut().find(|r| r.id == repo.id)
                {
                    main_repo.add_workspace(repo);
                } else {
                    ui_warning!(
                        ui,
                        "Unexpected workspace repository without a known \
                         default associated in {}",
                        root.display()
                    );
                }
            }
            Err(err) if err.downcast_ref::<NoRepositoryError>().is_some() => {
                search_dir(
                    ui,
                    config,
                    tree_space_kind,
                    repositories,
                    empty_dirs,
                    &root,
                );
            }
            // A repository which cannot be loaded is not to be explored as a
            // directory of the tree.
            Err(err) => ui_error!(
                ui,
                "Unable to load the repository {}: {err}",
                root.display()
            ),
        }
    }

    if empty_dir {
        empty_dirs.push(dir.to_path_buf());
    }
}

/// Search repositories in the repo tree.
fn search(ui: &Ui<'_>, config: &Config) -> (Vec<Repository>, Vec<PathBuf>) {
    let mut repositories = Vec::new();
    let mut empty_dirs = Vec::new();

    let entries = match config.root.read_dir() {
        Ok(entries) => entries,
        Err(err) => {
            ui_error!(
                ui,
                "Unable to read the repo tree root {}: {err}",
                config.root.display()
            );
            return (repositories, empty_dirs);
        }
    };

    for entry in entries.flatten() {
        let dir_path = entry.path();
        if let Some(tree_space) =
            TreeSpace::from_dir_name(config, &entry.file_name())
        {
            if tree_space.kind().is_workspace() {
                continue;
            }
            search_dir(
                ui,
                config,
                &tree_space.kind(),
                &mut repositories,
                &mut empty_dirs,
                &dir_path,
            );
        } else {
            ui_warning!(
                ui,
                "Unexpected tree-space directory: {}",
                dir_path.display(),
            );
        }
    }

    for tree_space in
        TreeSpace::iter().filter(|tree_space| tree_space.kind().is_workspace())
    {
        let dir_path = config.root.join(tree_space.category(config).dir_name());
        search_dir(
            ui,
            config,
            &tree_space.kind(),
            &mut repositories,
            &mut empty_dirs,
            &dir_path,
        );
    }

    for repository in &mut repositories {
        forget_missing_workspaces(ui, config, repository);
        apply_identity(ui, config, repository);
    }

    (repositories, empty_dirs)
}

/// Forget the workspaces of the repository which directory has been deleted
/// without telling the version control system, so that both agree on the list
/// of workspaces.
fn forget_missing_workspaces(
    ui: &Ui<'_>,
    config: &Config,
    repository: &Repository,
) {
    let Some(workspace) = repository.workspaces.first() else {
        return;
    };

    match repository
        .get_vcs_repo(workspace)
        .forget_missing_workspaces()
    {
        Ok(forgotten) => {
            for w in forgotten {
                ui_warning!(
                    ui,
                    "Forgot the workspace {} of the repository {}: its \
                     directory {} does not exist anymore",
                    w.name,
                    repository.id.display(config),
                    w.path.display()
                );
            }
        }
        Err(err) => {
            ui_error!(
                ui,
                "Unable to forget the missing workspaces of the repository \
                 {}: {}",
                repository.id.display(config),
                err
            );
        }
    }
}

/// Set the identity of the repository in its configuration, telling the
/// user what changed.
fn apply_identity(ui: &Ui<'_>, config: &Config, repository: &mut Repository) {
    match repository.apply_identity(config) {
        Ok(changes) => {
            if !changes.is_empty() {
                if let Err(err) = repository.update_tracked_files() {
                    ui_error!(
                        ui,
                        "Unable to track the files of the repository {}: {err}",
                        repository.id.display(config),
                    );
                }
                ui_hint!(
                    ui,
                    "Set the identity of the repository {}:\n  {}",
                    repository.id.display(config),
                    changes
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<String>>()
                        .join("\n  ")
                );
            }
        }
        Err(err) => {
            ui_error!(
                ui,
                "Unable to set the identity of the repository {}: {err}",
                repository.id.display(config),
            );
        }
    }
}

/// Track the configuration file, `None` when there is none.
fn track_config() -> Option<TrackedFile> {
    config_file()
        .ok()
        .and_then(|path| TrackedFile::new(path).ok())
}

/// Representation of the repository tree.
#[derive(Serialize, Deserialize)]
pub struct RepoTree {
    /// Configuration file the identities of the repositories were set from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    config: Option<TrackedFile>,
    /// List of repositories.
    repositories: Vec<Repository>,
}

impl RepoTree {
    /// Load the repository tree from the cache.
    fn from_cache() -> Result<Self, Box<dyn Error>> {
        let cache_file = cache_file()?;
        if cache_file.is_file() {
            Ok(toml::from_str::<Self>(&read_to_string(&cache_file)?)?)
        } else {
            Err(Box::new(NoCacheError()))
        }
    }

    /// Load all the repositories present in the repo tree.
    pub fn load_silent(
        ui: &Ui<'_>,
        config: &Config,
        refresh_cache: bool,
    ) -> Self {
        Self::load_silent_with_empty_dirs(ui, config, refresh_cache).0
    }

    /// Load all the repositories present in the repo tree with a list of
    /// detected empty directories within the repo tree. The list of empty
    /// directories, is returned only if the cache has not been used. As the
    /// cache exists to avoids us searching the repo tree, we should not do it
    /// anyway for getting the empty directories.
    pub fn load_silent_with_empty_dirs(
        ui: &Ui<'_>,
        config: &Config,
        refresh_cache: bool,
    ) -> (Self, Option<Vec<PathBuf>>) {
        if !refresh_cache {
            match Self::from_cache() {
                Ok(mut repo_tree) => {
                    if !repo_tree.repo_iter().any(Repository::has_been_modified)
                    {
                        let config_file = track_config();

                        // The identities may have changed.
                        if repo_tree.config != config_file {
                            repo_tree.apply_identities(ui, config);
                            repo_tree.config = config_file;
                            repo_tree.save_or_report(ui);
                        }

                        return (repo_tree, None);
                    }
                }
                // Not having a cache yet is expected, on the first run.
                Err(err) if err.downcast_ref::<NoCacheError>().is_some() => (),
                Err(err) => {
                    ui_error!(ui, "Failure to load the cache: {err}");
                }
            }
        }

        let (repo_tree, empty_dirs, result) = Self::search_and_save(ui, config);
        if let Err(err) = result {
            ui.error(err);
        }

        (repo_tree, Some(empty_dirs))
    }

    /// Search the repositories of the repo tree and write the cache. Returns
    /// the repositories, the empty directories found and the result of
    /// writing the cache.
    fn search_and_save(
        ui: &Ui<'_>,
        config: &Config,
    ) -> (Self, Vec<PathBuf>, Result<(), Box<dyn Error>>) {
        ui.hint("Refreshing repositories cache...");

        let config_file = track_config();
        let (repositories, empty_dirs) = search(ui, config);
        let repo_tree = Self {
            config: config_file,
            repositories,
        };
        let result = repo_tree.save();

        (repo_tree, empty_dirs, result)
    }

    /// Refresh the cache, warning about the empty directories found in the
    /// repo tree. Fails if the cache cannot be written.
    pub fn refresh(
        ui: &Ui<'_>,
        config: &Config,
    ) -> Result<Self, Box<dyn Error>> {
        let (repo_tree, empty_dirs, result) = Self::search_and_save(ui, config);
        warn_empty_dirs(ui, empty_dirs);
        result.map(|()| repo_tree)
    }

    /// Load the repo tree.
    /// Print a warning message if empty directories outside any repository are
    /// found in the repo tree.
    pub fn load(ui: &Ui<'_>, config: &Config, refresh_cache: bool) -> Self {
        let (repositories, empty_dirs) =
            Self::load_silent_with_empty_dirs(ui, config, refresh_cache);

        if let Some(empty_dirs) = empty_dirs {
            warn_empty_dirs(ui, empty_dirs);
        }

        repositories
    }

    /// Load some of the repositories based on the provided filters.
    pub fn filtered<'repo_tree>(
        &'repo_tree self,
        ui: &Ui<'_>,
        config: &Config,
        filter_hosts: &[Glob],
        filter_names: &[Glob],
        filter_trees: &[Glob],
    ) -> Vec<(&'repo_tree Repository, &'repo_tree Workspace)> {
        self.workspace_iter()
            .filter(|(r, w)| {
                (filter_hosts.is_empty()
                    || filter_hosts.iter().any(|host| {
                        match r.id.remote_host(config) {
                            Ok(Some(remote_host)) => host
                                .compile_matcher()
                                .is_match(&remote_host.category.name),
                            // Repositories without remote are matched
                            // as hosted by the local tree-space.
                            Ok(None) => host
                                .compile_matcher()
                                .is_match(&config.tree.local.category.name),
                            Err(err) => {
                                ui.error(err);
                                false
                            }
                        }
                    }))
                    && (filter_names.is_empty()
                        || filter_names.iter().any(|filter_name| {
                            filter_name.compile_matcher().is_match(&r.id.name)
                        }))
                    && (filter_trees.is_empty()
                        || filter_trees.iter().any(|filter_tree| {
                            if let Some(tree_space) = &w.tree_space() {
                                filter_tree
                                    .compile_matcher()
                                    .is_match(&tree_space.category(config).name)
                            } else {
                                false
                            }
                        }))
            })
            .collect()
    }

    /// Obtain an iterator on all the repositories.
    pub fn repo_iter<'repo_tree>(
        &'repo_tree self,
    ) -> Box<dyn Iterator<Item = &'repo_tree Repository> + 'repo_tree> {
        Box::new(self.repositories.iter())
    }

    /// Obtain an iterator on all the repositories workspaces.
    pub fn workspace_iter<'repo_tree>(
        &'repo_tree self,
    ) -> Box<
        dyn Iterator<Item = (&'repo_tree Repository, &'repo_tree Workspace)>
            + 'repo_tree,
    > {
        Box::new(
            self.repositories
                .iter()
                .flat_map(|r| r.workspaces.iter().map(move |w| (r, w))),
        )
    }

    /// Obtain the Repository and Workspace of the repository workspace at the
    /// provided path.
    pub fn get_workspace(
        &self,
        workspace_path: &Path,
    ) -> Option<(&Repository, &Workspace)> {
        self.workspace_iter()
            .find(|(_, w)| w.path() == workspace_path)
    }
}

/// Warn about each of the empty directories found in the repo tree.
fn warn_empty_dirs(ui: &Ui<'_>, empty_dirs: Vec<PathBuf>) {
    for empty_dir in empty_dirs {
        ui_warning!(
            ui,
            "Empty directory in repo tree: {}",
            empty_dir.display()
        );
    }
}

/// Path to the repositories cache file.
fn cache_file() -> Result<PathBuf, Box<dyn Error>> {
    cache_file_from(
        std::env::var_os("XDG_CACHE_HOME"),
        std::env::var_os("HOME"),
    )
}

/// Path to the repositories cache file from the values of the
/// `XDG_CACHE_HOME` and `HOME` environment variables.
///
/// As required by the XDG Base Directory specification, an empty or relative
/// `XDG_CACHE_HOME` is ignored.
fn cache_file_from(
    xdg_cache_home: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, Box<dyn Error>> {
    let base = match xdg_cache_home.map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => dir,
        _ => PathBuf::from(home.filter(|home| !home.is_empty()).ok_or(
            "cannot locate the cache file, neither XDG_CACHE_HOME nor HOME \
             environment variables are set",
        )?)
        .join(".cache"),
    };

    Ok(base.join("repo-tree").join("repo-tree.toml"))
}

impl RepoTree {
    /// Set the identities of all the repositories in their configuration.
    fn apply_identities(&mut self, ui: &Ui<'_>, config: &Config) {
        for repository in &mut self.repositories {
            apply_identity(ui, config, repository);
        }
    }

    /// Write the repository tree into the cache file, reporting a failure to
    /// the user.
    fn save_or_report(&self, ui: &Ui<'_>) {
        if let Err(err) = self.save() {
            ui.error(err);
        }
    }

    /// Write the repository tree into the cache file.
    ///
    /// The cache is written to a temporary file which then replaces the
    /// cache, so that a concurrent `rt` never reads a partially written
    /// cache.
    fn save(&self) -> Result<(), Box<dyn Error>> {
        let cache_file = cache_file()?;
        let write = || -> Result<(), Box<dyn Error>> {
            if let Some(parent) = cache_file.parent() {
                create_dir_all(parent)?;
            }

            let tmp_file = cache_file
                .with_extension(format!("toml.{}", std::process::id()));
            File::create(&tmp_file)?
                .write_all(toml::to_string(self)?.as_bytes())?;
            rename(&tmp_file, &cache_file)?;
            Ok(())
        };

        write().map_err(|err| {
            format!("Unable to write the cache {}: {err}", cache_file.display())
                .into()
        })
    }
}

#[cfg(test)]
impl RepoTree {
    /// Create a repository tree for tests from its repositories.
    pub fn from_repositories(repositories: Vec<Repository>) -> Self {
        Self {
            config: None,
            repositories,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::repo_id::RepoId;
    use crate::version_control_system::VersionControlSystem;

    /// Temporary repo tree, with the test configuration.
    struct Tree {
        /// Temporary directory of the tree.
        _dir: tempfile::TempDir,
        /// Configuration whose root is the temporary directory.
        config: Config,
    }

    impl Tree {
        /// Create an empty repo tree.
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("cannot create directory");
            let mut config = Config::test_default();
            config.root = dir.path().canonicalize().expect("canonical path");
            Self { _dir: dir, config }
        }

        /// Path of `path` in the tree.
        fn path(&self, path: &str) -> PathBuf {
            self.config.root.join(path)
        }

        /// Create a Git repository at `path` in the tree, with an `origin`
        /// remote if `url` is set.
        fn git_repo(&self, path: &str, url: Option<&str>) -> PathBuf {
            let path = self.path(path);
            let repo = git2::Repository::init(&path)
                .expect("cannot create repository");
            if let Some(url) = url {
                repo.remote("origin", url).expect("cannot add remote");
            }
            path
        }

        /// Search the repositories of the tree.
        fn search(&self) -> (Vec<Repository>, Vec<PathBuf>) {
            search(&Ui::new(&self.config), &self.config)
        }
    }

    #[test]
    fn search_finds_repositories_of_every_tree_space() {
        let tree = Tree::new();
        let bar =
            tree.git_repo("dev/test/foo/bar", Some("https://test.com/foo/bar"));
        let old =
            tree.git_repo("archive/test/foo/old", Some("git@test.com:foo/old"));
        let project = tree.git_repo("local/project", None);
        let agent = tree
            .git_repo("agent/test/foo/bar", Some("https://test.com/foo/bar"));

        let (mut repositories, empty_dirs) = tree.search();
        repositories.sort_by(|a, b| a.id.name.cmp(&b.id.name));

        assert_eq!(empty_dirs, Vec::<PathBuf>::new());
        let summary: Vec<(String, Vec<Workspace>)> = repositories
            .iter()
            .map(|r| (r.id.name.clone(), r.workspaces.clone()))
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "foo/bar".to_string(),
                    vec![
                        Workspace::Tree(TreeSpace::Dev, bar),
                        Workspace::Tree(TreeSpace::Agent, agent),
                    ]
                ),
                (
                    "foo/old".to_string(),
                    vec![Workspace::Tree(TreeSpace::Archive, old)]
                ),
                (
                    "project".to_string(),
                    vec![Workspace::Tree(TreeSpace::Local, project)]
                ),
            ]
        );
        assert!(
            repositories
                .iter()
                .all(|r| r.vcs == VersionControlSystem::Git)
        );
    }

    #[test]
    fn search_reports_empty_directories() {
        let tree = Tree::new();
        tree.git_repo("dev/test/foo/bar", Some("https://test.com/foo/bar"));
        fs::create_dir_all(tree.path("dev/test/empty/nested"))
            .expect("cannot create directory");
        fs::create_dir_all(tree.path("archive")).expect("cannot create dir");

        let (_, mut empty_dirs) = tree.search();
        empty_dirs.sort();

        assert_eq!(
            empty_dirs,
            [tree.path("archive"), tree.path("dev/test/empty/nested")]
        );
    }

    #[test]
    fn search_does_not_explore_unloadable_repositories() {
        let tree = Tree::new();
        // The remote URL cannot be parsed.
        let broken = tree.git_repo("dev/test/broken", Some("not a url"));
        fs::create_dir_all(broken.join("empty")).expect("cannot create dir");

        let (repositories, empty_dirs) = tree.search();

        assert_eq!(repositories, []);
        assert!(
            empty_dirs.iter().all(|dir| !dir.starts_with(&broken)),
            "{empty_dirs:?}"
        );
    }

    #[test]
    fn search_ignores_workspace_without_main_repository() {
        let tree = Tree::new();
        tree.git_repo(
            "agent/test/foo/orphan",
            Some("https://test.com/foo/orphan"),
        );

        let (repositories, _) = tree.search();

        assert_eq!(repositories, []);
    }

    #[test]
    fn search_without_root_finds_nothing() {
        let mut tree = Tree::new();
        tree.config.root = tree.path("missing");

        let (repositories, empty_dirs) = tree.search();

        assert_eq!(repositories, []);
        assert_eq!(empty_dirs, Vec::<PathBuf>::new());
    }

    #[test]
    fn repository_is_modified_when_a_workspace_is_removed() {
        let tree = Tree::new();
        let path = tree.git_repo("local/project", None);
        let repository =
            Repository::try_new(&tree.config, &path).expect("valid repository");
        assert!(!repository.has_been_modified());

        fs::remove_dir_all(&path).expect("cannot remove repository");

        assert!(repository.has_been_modified());
    }

    #[test]
    fn repository_is_modified_when_its_remote_changes() {
        let tree = Tree::new();
        let path = tree.git_repo("local/project", None);
        let repository =
            Repository::try_new(&tree.config, &path).expect("valid repository");
        let config = fs::File::options()
            .append(true)
            .open(path.join(".git/config"))
            .expect("cannot open configuration");
        config
            .set_modified(
                std::time::SystemTime::now()
                    + std::time::Duration::from_secs(5),
            )
            .expect("cannot set modification time");

        assert!(repository.has_been_modified());
    }

    #[test]
    fn cache_round_trips() {
        let tree = Tree::new();
        tree.git_repo("dev/test/foo/bar", Some("https://test.com/foo/bar"));
        tree.git_repo("agent/test/foo/bar", Some("https://test.com/foo/bar"));
        tree.git_repo("local/project", None);
        let repo_tree = RepoTree::from_repositories(tree.search().0);

        let serialized = toml::to_string(&repo_tree).expect("serializable");
        let loaded: RepoTree =
            toml::from_str(&serialized).expect("valid cache");

        assert_eq!(loaded.repositories, repo_tree.repositories);
    }

    #[test]
    fn cache_file_location() {
        let path = |xdg: Option<&str>, home: Option<&str>| {
            cache_file_from(xdg.map(Into::into), home.map(Into::into))
                .ok()
                .map(|p| p.display().to_string())
        };

        assert_eq!(
            path(Some("/xdg"), Some("/home/u")).as_deref(),
            Some("/xdg/repo-tree/repo-tree.toml")
        );
        assert_eq!(
            path(None, Some("/home/u")).as_deref(),
            Some("/home/u/.cache/repo-tree/repo-tree.toml")
        );
        assert_eq!(
            path(Some("relative"), Some("/home/u")).as_deref(),
            Some("/home/u/.cache/repo-tree/repo-tree.toml")
        );
        assert_eq!(path(None, None), None);
        assert_eq!(path(Some(""), Some("")), None);
    }

    /// Repository of the `test.com` host named `name`, with a workspace in
    /// each of `tree_spaces`.
    fn repo(name: &str, tree_spaces: &[TreeSpace]) -> Repository {
        let id = RepoId::from_remote_url(&format!("https://test.com/{name}"))
            .expect("valid URL");
        Repository::for_test(
            id,
            VersionControlSystem::Git,
            tree_spaces
                .iter()
                .map(|t| {
                    Workspace::Tree(
                        t.clone(),
                        PathBuf::from(format!("/{t:?}/{name}")),
                    )
                })
                .collect(),
        )
    }

    /// Workspaces of the test tree matching the filters.
    fn filtered(hosts: &[&str], names: &[&str], trees: &[&str]) -> Vec<String> {
        let config = Config::test_default();
        let ui = Ui::new(&config);
        let mut local = repo("x", &[TreeSpace::Local]);
        local.id.remote = None;
        local.id.name = "local-project".into();
        let repo_tree = RepoTree::from_repositories(vec![
            repo("owner/a", &[TreeSpace::Dev, TreeSpace::Agent]),
            repo("owner/b", &[TreeSpace::Archive]),
            repo("other/c", &[TreeSpace::Dev]),
            local,
        ]);
        let globs = |patterns: &[&str]| -> Vec<Glob> {
            patterns
                .iter()
                .map(|p| Glob::new(p).expect("valid glob"))
                .collect()
        };

        repo_tree
            .filtered(&ui, &config, &globs(hosts), &globs(names), &globs(trees))
            .into_iter()
            .map(|(_, w)| w.path().display().to_string())
            .collect()
    }

    #[test]
    fn filter_workspaces() {
        assert_eq!(filtered(&[], &[], &[]).len(), 5);
        assert_eq!(
            filtered(&["test"], &[], &[]),
            [
                "/Dev/owner/a",
                "/Agent/owner/a",
                "/Archive/owner/b",
                "/Dev/other/c"
            ]
        );
        assert_eq!(filtered(&["git*"], &[], &[]), Vec::<String>::new());
        assert_eq!(
            filtered(&[], &["owner/*"], &[]),
            ["/Dev/owner/a", "/Agent/owner/a", "/Archive/owner/b"]
        );
        assert_eq!(
            filtered(&[], &[], &["archive", "local"]),
            ["/Archive/owner/b", "/Local/x"]
        );
        assert_eq!(
            filtered(&[], &["*/a", "*/c"], &["dev"]),
            ["/Dev/owner/a", "/Dev/other/c"]
        );
    }

    #[test]
    fn get_workspace_by_path() {
        let repo_tree = RepoTree::from_repositories(vec![repo(
            "owner/a",
            &[TreeSpace::Dev, TreeSpace::Agent],
        )]);

        let (repository, workspace) = repo_tree
            .get_workspace(Path::new("/Agent/owner/a"))
            .expect("known workspace");
        assert_eq!(repository.id.name, "owner/a");
        assert_eq!(workspace.tree_space(), Some(&TreeSpace::Agent));
        assert!(repo_tree.get_workspace(Path::new("/Agent/owner")).is_none());
    }
}
