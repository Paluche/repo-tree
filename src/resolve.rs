//! Resolve a repository identifier argument into a Repository.
use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::io::Write;
use std::iter::zip;
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

/// Find the shortest end-path to identify two path.
fn reduce(path_a: &str, path_b: &str) -> Option<(String, String)> {
    let mut ret_a = Vec::new();
    let mut ret_b = Vec::new();
    for (a, b) in zip(path_a.split('/').rev(), path_b.split('/').rev()) {
        ret_a.insert(0, a);
        ret_b.insert(0, b);

        if a != b {
            return Some((ret_a.join("/"), ret_b.join("/")));
        }
    }

    None
}

/// Get the workspace associated with the repository candidate. Based on the
/// filtering.
fn get_workspace<'repo_tree>(
    repository: &'repo_tree Repository,
    maybe_tree_space: Option<&TreeSpace>,
) -> Option<&'repo_tree Workspace> {
    match maybe_tree_space {
        Some(tree_space) => repository.get_tree_workspace(tree_space),
        None => Some(repository.get_main_workspace()),
    }
}

/// Reduce the name of the repositories to the shortest path that identifies
/// each repositories individually.
fn reduce_repo_names<'repo_tree>(
    config: &Config,
    repo_tree: &'repo_tree RepoTree,
    maybe_tree_space: Option<&TreeSpace>,
) -> BTreeMap<String, &'repo_tree Repository> {
    let mut ret: BTreeMap<String, &Repository> = BTreeMap::new();

    for repository in repo_tree.repo_iter() {
        let Some(workspace) = get_workspace(repository, maybe_tree_space)
        else {
            continue;
        };

        let name = repository.id.name.clone();
        if let Some(full_name) = repository
            .id
            .remote_host_name(config)
            .map(|remote_host_name| format!("{remote_host_name}/{name}"))
        {
            if let Some(conflict_repository) = ret.remove(&full_name) {
                let conflict_workspace =
                    get_workspace(conflict_repository, maybe_tree_space)
                        .unwrap();

                eprintln!(
                    "Duplicated repository with name {name}: {0} and {1}\n
                    {1} is ignored!",
                    conflict_workspace.path().display(),
                    workspace.path().display(),
                );
                continue;
            }
            ret.insert(full_name, repository);
        }
        let short_name = String::from(name.split('/').next_back().unwrap());

        if name != short_name {
            ret.insert(name.clone(), repository);
        }

        if let Some(conflict_repository) = ret.remove(&short_name) {
            if let Some((conflict_reduced_name, reduced_name)) =
                reduce(&conflict_repository.id.name, &name)
            {
                ret.insert(conflict_reduced_name, conflict_repository);
                ret.insert(reduced_name, repository);
            }
        } else {
            ret.insert(short_name, repository);
        }
    }

    ret
}

/// Get the map associating valid repository identifiers to the associated
/// repository present in the repo tree.
fn get_candidates<'repo_tree>(
    config: &Config,
    repo_tree: &'repo_tree RepoTree,
    maybe_tree_space: Option<&TreeSpace>,
) -> BTreeMap<String, &'repo_tree Repository> {
    let mut ret = reduce_repo_names(config, repo_tree, maybe_tree_space);

    for (alias, repo_name) in config.command.resolve.aliases.iter() {
        if let Some(repo) = ret.get(repo_name) {
            ret.insert(alias.to_string(), repo);
        } else if maybe_tree_space.is_none() {
            // This warning is no more reliable when there is filtering. So
            // print it only if there is no filtering.
            eprintln!(
                "Configured alias \"{alias}\" => \"{repo_name}\", does not \
                 correspond to any existing repository."
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
        stdin.write_all(&candidates.join("\n").into_bytes())?
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
    repository: &Repository,
    config: &Config,
) -> Option<StyledStr> {
    repository.id.remote.as_ref().map(|r| {
        StyledStr::from(format!(
            "{}{}",
            r.url,
            if let Some(tree_space) =
                &repository.get_main_workspace().tree_space()
                && !matches!(tree_space, TreeSpace::Dev)
            {
                format!(" <{}>", tree_space.display(config))
            } else {
                "".to_string()
            }
        ))
    })
}

/// Resolve a repository identifier into a local repository.
pub fn resolve_repo<'repo_tree>(
    config: &Config,
    repo_tree: &'repo_tree RepoTree,
    repo_id: Option<String>,
    maybe_tree_space: Option<&TreeSpace>,
) -> Result<Option<&'repo_tree Repository>, Box<dyn Error>> {
    let mut candidates = get_candidates(config, repo_tree, maybe_tree_space);

    if candidates.is_empty() {
        eprintln!(
            "No repository in {}",
            if let Some(tree_space) = &maybe_tree_space {
                format!("tree space {}", tree_space.display(config))
            } else {
                "repo-tree".to_string()
            }
        );
        return Ok(None);
    }

    let repo_id = match repo_id {
        Some(repo_id) => repo_id,
        None => match fzf_ask(candidates.keys())? {
            Some(repo_id) => repo_id,
            None => {
                eprintln!("Nothing selected");
                return Ok(None);
            }
        },
    };

    let repo = candidates.remove(&repo_id);

    if repo.is_some() {
        return Ok(repo);
    }

    let matcher = SkimMatcherV2::default();

    let mut matches: Vec<_> = candidates
        .keys()
        .filter_map(|item| {
            matcher
                .fuzzy_match(item, &repo_id)
                .map(|score| (score, item))
        })
        .collect();

    if matches.is_empty() {
        eprintln!("No match for {repo_id}");
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
        eprintln!("Considering you meant {name}");
        Ok(Some(repo))
    } else {
        eprintln!("Several possible match:");

        let mut matches = matches.iter();

        for (name, repo) in matches.by_ref().take(8) {
            eprint!("- {name}");
            if let Some(help) = repository_candidate_help(repo, config) {
                eprintln!(" -> {}", help);
            }
        }

        let remains = matches.count();

        if remains != 0 {
            eprintln!("...");
            eprintln!(
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
    config: &Config,
    repo_tree: &'repo_tree RepoTree,
    repo_id: Option<String>,
    maybe_tree_space: Option<&TreeSpace>,
) -> Result<
    Option<(&'repo_tree Repository, &'repo_tree Workspace)>,
    Box<dyn Error>,
> {
    if let Some(repo) =
        resolve_repo(config, repo_tree, repo_id, maybe_tree_space)?
    {
        let workspace =
            if maybe_tree_space.is_none() && repo.workspaces.len() != 1 {
                if let Some(tree_space) = fzf_ask(
                    repo.workspaces
                        .iter()
                        .map(|w| w.tree_space().unwrap().name(config)),
                )?
                .map(|name| TreeSpace::from_name(config, &name).unwrap())
                {
                    repo.get_tree_workspace(&tree_space).unwrap()
                } else {
                    eprintln!(
                        "No tree space specified, defaulting to the default \
                         workspace"
                    );
                    repo.get_main_workspace()
                }
            } else {
                get_workspace(repo, maybe_tree_space).unwrap()
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
        let repo_tree = RepoTree::load_silent(&config, false);
        let candidates = get_candidates(&config, &repo_tree, None);
        let matcher = SkimMatcherV2::default();

        candidates
            .keys()
            .filter_map(|item| {
                matcher.fuzzy_match(item, current).map(|_| {
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
                        .help(repository_candidate_help(repository, &config))
                })
            })
            .collect::<Vec<CompletionCandidate>>()
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_reduce_same_length() {
        assert_eq!(
            reduce("foo/bar/dead/beaf/toutidou", "foo/bar/deaf/beaf/toutidou",),
            Some((
                "dead/beaf/toutidou".to_string(),
                "deaf/beaf/toutidou".to_string(),
            ))
        )
    }

    #[test]
    fn test_reduce_different_length_diff() {
        assert_eq!(
            reduce("foo/bar/dead/beaf/toutidou", "deaf/beaf/toutidou",),
            Some((
                "dead/beaf/toutidou".to_string(),
                "deaf/beaf/toutidou".to_string(),
            ))
        )
    }

    #[test]
    fn test_reduce_min_length_match() {
        assert_eq!(
            reduce("foo/bar/dead/beaf/toutidou", "dead/beaf/toutidou",),
            None,
            // TODO Should it actually be the following?
            // Some((
            //     "bar/dead/beaf/toutidou".to_string(),
            //     "deaf/beaf/toutidou".to_string(),
            // ))
        )
    }
}
