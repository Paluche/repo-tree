//! Resolve a repository identifier argument into a Repository.
use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use clap::builder::StyledStr;
use clap_complete::engine::ArgValueCompleter;
use clap_complete::engine::CompletionCandidate;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use which::which;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::repository::Repository;
use crate::repository::Workspace;
use crate::tree_space::TreeSpace;
use crate::tree_space::TreeSpaceKind;
use crate::ui::Ui;
use crate::ui_error;
use crate::ui_hint;
use crate::ui_warning;

/// All potential filter options for the `resolve()` function.
#[derive(Default)]
pub enum ResolveFilter {
    /// No filter.
    #[default]
    All,
    /// Filter based on a specific tree-space. Expecting the repository to
    /// resolve to, to belong the specified tree-space.
    TreeSpace(TreeSpace),
    /// Filter based on a kind of tree-space. Expecting the repository to
    /// resolve to, to belong in a tree-space of the specified kind.
    TreeSpaceKind(TreeSpaceKind),
}

impl ResolveFilter {
    /// Find out if the value is the All variant.
    fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Get the enum value associated with the CLI arguments association.
    pub fn from_cli_args(
        tree: Option<TreeSpace>,
        kind: Option<TreeSpaceKind>,
    ) -> Result<Self, ()> {
        match (tree, kind) {
            (Some(_), Some(_)) => Err(()),
            (Some(t), None) => Ok(Self::TreeSpace(t)),
            (None, Some(k)) => Ok(Self::TreeSpaceKind(k)),
            (None, None) => Ok(Self::All),
        }
    }
}

/// Get the workspace associated with the repository candidate. Based on the
/// filtering.
fn get_workspace<'repo_tree>(
    ui: &Ui<'_>,
    repository: &'repo_tree Repository,
    filter: &ResolveFilter,
) -> Option<&'repo_tree Workspace> {
    match filter {
        ResolveFilter::All => Some(repository.get_main_workspace(ui)),
        ResolveFilter::TreeSpace(tree_space) => {
            repository.get_tree_workspace(ui, tree_space)
        }
        ResolveFilter::TreeSpaceKind(tree_space_kind) => {
            repository.workspaces.iter().find_map(|w| {
                w.tree_space()
                    .and_then(|t| (&t.kind() == tree_space_kind).then_some(w))
            })
        }
    }
}

/// Shortest suffix of the path components `name` which is not a suffix of
/// any of the `others` path components. `None` if there is none, which is the
/// case when `name` is itself a suffix of one of the `others`.
fn unique_suffix(name: &[&str], others: &[Vec<&str>]) -> Option<String> {
    (1..=name.len())
        .map(|length| &name[name.len() - length..])
        .find(|suffix| !others.iter().any(|other| other.ends_with(suffix)))
        .map(|suffix| suffix.join("/"))
}

/// Associate to the repositories the identifiers they can be resolved with:
/// - their name prefixed by the name of their host (e.g. `github/owner/repo`),
/// - their name (e.g. `owner/repo`),
/// - the shortest end of their name which identifies them (e.g. `repo`).
fn reduce_repo_names<'repo_tree>(
    ui: &Ui<'_>,
    config: &Config,
    repo_tree: &'repo_tree RepoTree,
    filter: &ResolveFilter,
) -> BTreeMap<String, &'repo_tree Repository> {
    // Keep the first repository of each full name.
    let mut repositories: Vec<(String, &Repository, &Workspace)> = Vec::new();
    for repository in repo_tree.repo_iter() {
        let Some(workspace) = get_workspace(ui, repository, filter) else {
            continue;
        };
        let name = &repository.id.name;
        let full_name = repository
            .id
            .remote_host_name(config)
            .map_or_else(|| name.clone(), |host| format!("{host}/{name}"));

        if let Some((_, _, first)) =
            repositories.iter().find(|(n, _, _)| *n == full_name)
        {
            ui_warning!(
                ui,
                "Duplicated repository with name {full_name}: {} and {}, the \
                 latter is ignored",
                first.path().display(),
                workspace.path().display(),
            );
            continue;
        }
        repositories.push((full_name, repository, workspace));
    }

    let names: Vec<Vec<&str>> = repositories
        .iter()
        .map(|(_, repository, _)| repository.id.name.split('/').collect())
        .collect();

    let mut ret = BTreeMap::new();

    // Short names first, so that they never hide the name of a repository.
    for (index, (_, repository, _)) in repositories.iter().enumerate() {
        let others: Vec<Vec<&str>> = names
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, name)| name.clone())
            .collect();
        if let Some(short_name) = unique_suffix(&names[index], &others) {
            ret.insert(short_name, *repository);
        }
    }
    for (index, (full_name, repository, _)) in repositories.iter().enumerate() {
        // The name alone is ambiguous if several hosts have a repository with
        // that name.
        if !names
            .iter()
            .enumerate()
            .any(|(i, name)| i != index && *name == names[index])
        {
            ret.insert(repository.id.name.clone(), *repository);
        }
        ret.insert(full_name.clone(), *repository);
    }

    ret
}

/// Get the map associating valid repository identifiers to the associated
/// repository present in the repo tree.
fn get_candidates<'repo_tree>(
    ui: &Ui<'_>,
    config: &Config,
    repo_tree: &'repo_tree RepoTree,
    filter: &ResolveFilter,
) -> BTreeMap<String, &'repo_tree Repository> {
    let mut ret = reduce_repo_names(ui, config, repo_tree, filter);

    for (alias, repo_name) in &config.command.resolve.aliases {
        if let Some(repo) = ret.get(repo_name) {
            ret.insert(alias.clone(), repo);
        } else if filter.is_all() {
            // This warning is no more reliable when there is filtering. So
            // print it only if there is no filtering.
            ui_warning!(
                ui,
                "Configured alias \"{alias}\" => \"{repo_name}\", does not \
                 correspond to any existing repository.",
            );
        }
    }

    ret
}

/// Interactively ask the user to select the repository.
fn fzf_ask<'c, I>(mut candidates: I) -> Result<Option<String>, Box<dyn Error>>
where
    I: Iterator<Item = &'c String>,
{
    let fzf = which("fzf")?;

    let mut child = Command::new(fzf)
        .arg("--preview")
        .arg(
            "rt repo state --color always --verbose --repository \"$(rt \
             resolve {})\"",
        )
        .arg("--preview-label=STATE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // Provide choices on stdin.
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(&candidates.join("\n").into_bytes())?;
    }

    // Wait and read selection.
    let output = child.wait_with_output()?;
    let res = String::from_utf8_lossy(&output.stdout).into_owned();
    let res = if let Some(res) = res.strip_suffix('\n') {
        res.to_string()
    } else {
        res
    };

    if res.is_empty() {
        Ok(None)
    } else {
        Ok(Some(res))
    }
}

/// Get the text that describe the associated repository ID completion
/// candidate.
fn repository_candidate_help(
    ui: &Ui<'_>,
    config: &Config,
    repository: &Repository,
) -> Option<StyledStr> {
    repository.id.remote.as_ref().map(|r| {
        StyledStr::from(format!(
            "{}{}",
            r.url,
            if let Some(tree_space) =
                &repository.get_main_workspace(ui).tree_space()
                && !matches!(tree_space, TreeSpace::Dev)
            {
                format!(" <{}>", tree_space.display(config))
            } else {
                String::new()
            }
        ))
    })
}

/// Resolve a repository identifier into a local repository.
pub fn resolve_repo<'repo_tree>(
    ui: &Ui<'_>,
    config: &Config,
    repo_tree: &'repo_tree RepoTree,
    repo_id: Option<String>,
    filter: &ResolveFilter,
) -> Result<Option<&'repo_tree Repository>, Box<dyn Error>> {
    let mut candidates = get_candidates(ui, config, repo_tree, filter);

    if candidates.is_empty() {
        ui_error!(
            ui,
            "No repository in {}",
            match filter {
                ResolveFilter::All => "repo-tree".to_string(),
                ResolveFilter::TreeSpace(tree_space) =>
                    format!("tree space {}", tree_space.display(config)),
                ResolveFilter::TreeSpaceKind(tree_space_kind) =>
                    format!("tree spaces of kind {tree_space_kind}"),
            }
        );
        return Ok(None);
    }

    let repo_id = if let Some(repo_id) = repo_id {
        repo_id
    } else if let Some(repo_id) = fzf_ask(candidates.keys())? {
        repo_id
    } else {
        ui.error("Nothing selected");
        return Ok(None);
    };

    let repo = candidates.remove(&repo_id);

    if repo.is_some() {
        return Ok(repo);
    }

    let fuzzy_matcher = SkimMatcherV2::default();

    let mut matches: Vec<_> = candidates
        .keys()
        .filter_map(|item| {
            fuzzy_matcher
                .fuzzy_match(item, &repo_id)
                .map(|score| (score, item))
        })
        .collect();

    if matches.is_empty() {
        ui_error!(ui, "No match for {repo_id}");
        return Ok(None);
    }

    // Sort by match score.
    matches.sort_by_key(|i| std::cmp::Reverse(i.0));
    // Remove matches that would lead to the same result.
    let matches: Vec<(&str, &Repository)> = {
        let mut res: Vec<(&str, &Repository)> = Vec::new();

        for (name, repo) in matches
            .into_iter()
            .map(|(_, name)| (name, candidates.get(name).unwrap()))
        {
            if res.iter().any(|(_, r)| r == repo) {
                continue;
            }
            res.push((name, repo));
        }

        res
    };

    if matches.len() == 1 {
        let (name, repo) = matches[0];
        ui_hint!(ui, "Considering you meant {name}");
        Ok(Some(repo))
    } else {
        ui.hint("Several possible match:");

        let mut matches = matches.iter();

        for (name, repo) in matches.by_ref().take(8) {
            ui_hint!(ui, "- {name}");
            if let Some(help) = repository_candidate_help(ui, config, repo) {
                ui_hint!(ui, " -> {}", help);
            }
        }

        let remains = matches.count();

        if remains != 0 {
            ui.hint("...");
            ui_hint!(
                ui,
                "{} more possibilit{}.",
                remains,
                if remains == 1 { "y" } else { "ies" }
            );
        }

        Ok(None)
    }
}

/// Resolve a repository identifier into a local repository.
pub fn resolve<'repo_tree>(
    ui: &Ui<'_>,
    config: &Config,
    repo_tree: &'repo_tree RepoTree,
    repo_id: Option<String>,
    filter: &ResolveFilter,
) -> Result<
    Option<(&'repo_tree Repository, &'repo_tree Workspace)>,
    Box<dyn Error>,
> {
    if let Some(repo) = resolve_repo(ui, config, repo_tree, repo_id, filter)? {
        let workspace = if filter.is_all() && repo.workspaces.len() != 1 {
            let names = repo
                .workspaces
                .iter()
                .filter_map(Workspace::tree_space)
                .map(|tree_space| tree_space.name(config));
            if let Some(name) = fzf_ask(names)? {
                let tree_space = TreeSpace::from_name(config, &name)
                    .ok_or_else(|| format!("Unknown tree-space {name:?}"))?;
                repo.get_tree_workspace(ui, &tree_space).ok_or_else(|| {
                    format!(
                        "The repository {} has no workspace in the {name} \
                         tree-space",
                        repo.id.name
                    )
                })?
            } else {
                ui.hint(
                    "No tree space specified, defaulting to the default \
                     workspace",
                );
                repo.get_main_workspace(ui)
            }
        } else {
            get_workspace(ui, repo, filter).expect(
                "resolve_repo only returns repositories with a workspace \
                 matching the filter",
            )
        };
        Ok(Some((repo, workspace)))
    } else {
        Ok(None)
    }
}

/// Get auto-completion candidate for a repository identifier argument.
pub fn resolve_completer() -> ArgValueCompleter {
    ArgValueCompleter::new(|current: &OsStr| {
        let Some(current) = current.to_str() else {
            return vec![];
        };
        let Ok(config) = Config::load() else {
            return vec![];
        };
        let ui = Ui::new(&config);
        let repo_tree = RepoTree::load_silent(&ui, &config, false);
        let candidates =
            get_candidates(&ui, &config, &repo_tree, &ResolveFilter::All);
        let fuzzy_matcher = SkimMatcherV2::default();

        candidates
            .keys()
            .filter_map(|item| {
                fuzzy_matcher.fuzzy_match(item, current).map(|_| {
                    let repository = candidates.get(item).unwrap();

                    CompletionCandidate::new(item)
                        .tag(
                            repository
                                .id
                                .remote_host(&config)
                                .ok()
                                .flatten()
                                .map(|r| {
                                    StyledStr::from(
                                        r.category.dir_name().to_string(),
                                    )
                                }),
                        )
                        .help(repository_candidate_help(
                            &ui, &config, repository,
                        ))
                })
            })
            .collect::<Vec<CompletionCandidate>>()
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::repo_id::RepoId;
    use crate::version_control_system::VersionControlSystem;

    /// Repository cloned from `url`, with a workspace in `tree_space`.
    fn repo(url: &str, tree_space: TreeSpace) -> Repository {
        let id = RepoId::from_remote_url(url).expect("valid URL");
        let path = PathBuf::from("/home/user/work")
            .join(format!("{tree_space:?}").to_lowercase())
            .join(&id.name);
        Repository::for_test(
            id,
            VersionControlSystem::Git,
            vec![Workspace::Tree(tree_space, path)],
        )
    }

    /// Local repository named `name`.
    fn local(name: &str) -> Repository {
        let path = PathBuf::from("/home/user/work/local").join(name);
        let id = RepoId::from_repo(&path, None).expect("valid name");
        Repository::for_test(
            id,
            VersionControlSystem::Git,
            vec![Workspace::Tree(TreeSpace::Local, path)],
        )
    }

    /// Identifiers of each repository of the tree, as `identifier =>
    /// repository name`.
    fn identifiers(
        config: &Config,
        repositories: Vec<Repository>,
    ) -> Vec<String> {
        let ui = Ui::new(config);
        let tree = RepoTree::from_repositories(repositories);
        get_candidates(&ui, config, &tree, &ResolveFilter::All)
            .into_iter()
            .map(|(key, repo)| format!("{key} => {}", repo.id.name))
            .collect()
    }

    #[test]
    fn identifiers_of_unique_repositories() {
        assert_eq!(
            identifiers(
                &Config::test_default(),
                vec![
                    repo("https://test.com/foo/bar", TreeSpace::Dev),
                    local("project"),
                ]
            ),
            [
                "bar => foo/bar",
                "foo/bar => foo/bar",
                "project => project",
                "test/foo/bar => foo/bar",
            ]
        );
    }

    #[test]
    fn identifiers_of_repositories_with_same_short_name() {
        assert_eq!(
            identifiers(
                &Config::test_default(),
                vec![
                    repo("https://test.com/a/x/repo", TreeSpace::Dev),
                    repo("https://test.com/b/x/repo", TreeSpace::Dev),
                    repo("https://test.com/c/repo", TreeSpace::Dev),
                ]
            ),
            [
                "a/x/repo => a/x/repo",
                "b/x/repo => b/x/repo",
                "c/repo => c/repo",
                "test/a/x/repo => a/x/repo",
                "test/b/x/repo => b/x/repo",
                "test/c/repo => c/repo",
            ]
        );
    }

    #[test]
    fn identifiers_of_local_repository_named_as_another_one() {
        assert_eq!(
            identifiers(
                &Config::test_default(),
                vec![
                    repo("https://test.com/foo/repo", TreeSpace::Dev),
                    local("repo"),
                ]
            ),
            [
                "foo/repo => foo/repo",
                "repo => repo",
                "test/foo/repo => foo/repo",
            ]
        );
    }

    #[test]
    fn identifiers_of_same_repository_on_two_hosts() {
        assert_eq!(
            identifiers(
                &Config::test_default(),
                vec![
                    repo("https://test.com/foo/bar", TreeSpace::Dev),
                    repo("https://github.com/foo/bar", TreeSpace::Dev),
                ]
            ),
            ["github/foo/bar => foo/bar", "test/foo/bar => foo/bar"]
        );
    }

    #[test]
    fn duplicated_repository_keeps_the_first() {
        let config = Config::test_default();
        let ui = Ui::new(&config);
        let first = repo("https://test.com/foo/bar", TreeSpace::Dev);
        let second = repo("https://test.com/foo/bar", TreeSpace::Archive);
        let tree = RepoTree::from_repositories(vec![first.clone(), second]);

        let candidates =
            get_candidates(&ui, &config, &tree, &ResolveFilter::All);

        assert!(candidates.values().all(|r| **r == first));
        assert_eq!(candidates.len(), 3);
    }

    #[test]
    fn aliases_point_to_existing_identifiers() {
        let mut config = Config::test_default();
        config
            .command
            .resolve
            .aliases
            .insert("b".into(), "foo/bar".into());
        config
            .command
            .resolve
            .aliases
            .insert("missing".into(), "nothing".into());

        assert_eq!(
            identifiers(
                &config,
                vec![repo("https://test.com/foo/bar", TreeSpace::Dev)]
            ),
            [
                "b => foo/bar",
                "bar => foo/bar",
                "foo/bar => foo/bar",
                "test/foo/bar => foo/bar",
            ]
        );
    }

    #[test]
    fn filter_on_tree_space() {
        let config = Config::test_default();
        let ui = Ui::new(&config);
        let tree = RepoTree::from_repositories(vec![
            repo("https://test.com/foo/bar", TreeSpace::Dev),
            repo("https://test.com/foo/old", TreeSpace::Archive),
        ]);

        let names: Vec<String> = get_candidates(
            &ui,
            &config,
            &tree,
            &ResolveFilter::TreeSpace(TreeSpace::Archive),
        )
        .into_values()
        .map(|r| r.id.name.clone())
        .collect();

        assert_eq!(names, ["foo/old"; 3]);
    }

    #[test]
    fn resolve_filter_from_cli_args() {
        assert!(matches!(
            ResolveFilter::from_cli_args(None, None),
            Ok(ResolveFilter::All)
        ));
        assert!(matches!(
            ResolveFilter::from_cli_args(Some(TreeSpace::Dev), None),
            Ok(ResolveFilter::TreeSpace(TreeSpace::Dev))
        ));
        assert!(matches!(
            ResolveFilter::from_cli_args(None, Some(TreeSpaceKind::Main)),
            Ok(ResolveFilter::TreeSpaceKind(TreeSpaceKind::Main))
        ));
        assert!(
            ResolveFilter::from_cli_args(
                Some(TreeSpace::Dev),
                Some(TreeSpaceKind::Main)
            )
            .is_err()
        );
    }

    /// Resolve `id` in a tree made of a few repositories.
    fn resolve_id(id: &str) -> Option<String> {
        let config = Config::test_default();
        let ui = Ui::new(&config);
        let tree = RepoTree::from_repositories(vec![
            repo("https://test.com/owner/rocket", TreeSpace::Dev),
            repo("https://test.com/owner/pocket", TreeSpace::Dev),
            repo("https://test.com/other/tools", TreeSpace::Dev),
        ]);

        resolve_repo(&ui, &config, &tree, Some(id.into()), &ResolveFilter::All)
            .expect("no error")
            .map(|r| r.id.name.clone())
    }

    #[test]
    fn resolve_exact_identifier() {
        assert_eq!(resolve_id("rocket").as_deref(), Some("owner/rocket"));
        assert_eq!(
            resolve_id("test/other/tools").as_deref(),
            Some("other/tools")
        );
    }

    #[test]
    fn resolve_single_fuzzy_match() {
        assert_eq!(resolve_id("tls").as_deref(), Some("other/tools"));
    }

    #[test]
    fn resolve_ambiguous_or_unknown_identifier() {
        assert_eq!(resolve_id("ocket"), None);
        assert_eq!(resolve_id("zzz"), None);
    }

    #[test]
    fn resolve_in_empty_tree() {
        let config = Config::test_default();
        let ui = Ui::new(&config);
        let tree = RepoTree::from_repositories(vec![]);

        assert!(
            resolve_repo(
                &ui,
                &config,
                &tree,
                Some("x".into()),
                &ResolveFilter::All
            )
            .expect("no error")
            .is_none()
        );
    }
}
