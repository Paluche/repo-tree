//! Resolve a repository identifier argument into a Repository.
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::iter::zip;
use std::process::Command;
use std::process::Stdio;

use clap::builder::StyledStr;
use clap_complete::engine::CompletionCandidate;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use which::which;

use crate::config::Config;
use crate::repository::Repositories;
use crate::repository::Repository;

/// Find the shortest end-path to identify two path.
fn reduce(path_a: &str, path_b: &str) -> Option<(String, String)> {
    let mut ret_a = Vec::new();
    let mut ret_b = Vec::new();
    for (a, b) in zip(
        path_a.split('/').collect::<Vec<&str>>(),
        path_b.split('/').collect::<Vec<&str>>(),
    )
    .rev()
    {
        ret_a.insert(0, a);
        ret_b.insert(0, b);
        if a != b {
            break;
        }
    }

    if ret_a != ret_b {
        Some((ret_a.join("/"), ret_b.join("/")))
    } else {
        None
    }
}

/// Reduce the name of the repositories to the shortest path that identifies
/// each repositories individually.
fn reduce_repo_names<'repos>(
    config: &Config,
    repositories: &'repos Repositories,
) -> HashMap<String, &'repos Repository> {
    let mut ret: HashMap<String, &Repository> = HashMap::new();

    for repository in repositories.iter() {
        let name = repository.id.name.clone();
        if let Ok(full_name) = repository
            .id
            .remote
            .host(config)
            .name()
            .inspect_err(|err| eprintln!("{err}"))
            .map(|host_name| format!("{host_name}/{name}"))
        {
            if let Some(conflict) = ret.remove(&full_name) {
                eprintln!(
                    "Duplicated repository with name {name}: {0} and {1}\n
                    {1} is ignored!",
                    conflict.root.display(),
                    repository.root.display(),
                );
            }
            ret.insert(full_name, repository);
        }
        let short_name = String::from(name.split('/').next_back().unwrap());

        if name != short_name {
            ret.insert(name.clone(), repository);
        }

        if let Some(conflict) = ret.remove(&short_name) {
            if let Some((conflict_reduced_name, reduced_name)) =
                reduce(&conflict.id.name, &name)
            {
                ret.insert(conflict_reduced_name, conflict);
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
fn get_candidates<'repos>(
    config: &Config,
    repositories: &'repos Repositories,
) -> HashMap<String, &'repos Repository> {
    let mut ret = reduce_repo_names(config, repositories);

    for (alias, repo_name) in config.command.resolve.aliases.iter() {
        if let Some(repo) = ret.get(repo_name) {
            ret.insert(alias.to_string(), repo);
        } else {
            eprintln!(
                "Configured alias \"{alias}\" => \"{repo_name}\", does not \
                 correspond to any existing repository."
            );
        }
    }

    ret
}

/// Interactively ask the user to select the repository.
fn fzf_ask(
    repositories: &HashMap<String, &Repository>,
) -> Result<String, Box<dyn Error>> {
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
        stdin.write_all(&repositories.keys().join("\n").into_bytes())?
    }

    // Wait and read selection.
    let output = child.wait_with_output()?;
    let res = String::from_utf8_lossy(&output.stdout).into_owned();
    if let Some(res) = res.strip_suffix('\n') {
        Ok(res.to_string())
    } else {
        Ok(res)
    }
}

/// Resolve a repository identifier into a local repository.
pub fn resolve<'repos>(
    config: &Config,
    repositories: &'repos Repositories,
    repo_id: Option<String>,
) -> Result<Option<&'repos Repository>, Box<dyn Error>> {
    let mut candidates = get_candidates(config, repositories);

    let repo_id = match repo_id {
        Some(repo_id) => repo_id,
        None => fzf_ask(&candidates)?,
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

    matches
        .dedup_by_key(|(_, name)| candidates.get(*name).unwrap().root.to_str());

    if matches.len() == 1 {
        let name = matches[0].1;
        eprintln!("Considering you meant {name}");
        Ok(Some(candidates.get(name).unwrap()))
    } else {
        eprintln!("Several possible match:");
        // Sort by match score.
        matches.sort_by_key(|i| std::cmp::Reverse(i.0));

        for (_, name) in matches {
            eprintln!("- {name}");
        }

        Ok(None)
    }
}

/// Get auto-completion candidate for a repository identifier argument.
pub fn resolve_completer(
    current: &std::ffi::OsStr,
) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };
    let Ok(config) = Config::load() else {
        return vec![];
    };
    let repositories = Repositories::load_silent(&config, false);
    let candidates = get_candidates(&config, &repositories);
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
                            .remote
                            .host(&config)
                            .dir_name()
                            .ok()
                            .map(StyledStr::from),
                    )
                    .help(repository.id.remote.url().map(StyledStr::from))
            })
        })
        .collect()
}
