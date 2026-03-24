//! Action to resolve the path to a repository from its name or alias.
use std::{
    collections::HashMap,
    io::Write,
    iter::zip,
    process::{Command, Stdio},
};

use clap::{Args, builder::StyledStr};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use itertools::Itertools;
use which::which;

use crate::{Config, Repository, load_repositories};

/// Resolve the name of a repository into its path.
#[derive(Args, Debug, PartialEq)]
pub struct ResolveArgs {
    /// Repository identifier to resolve into the actual path within the
    /// repo_tree.
    #[arg(add=ArgValueCompleter::new(resolve_completer))]
    repo_id: Option<String>,
}

/// Find the shortest end-path to identify two
fn reduce(path_a: String, path_b: String) -> Option<(String, String)> {
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
fn reduce_repo_names(
    repositories: Vec<Repository>,
) -> HashMap<String, Repository> {
    let mut ret: HashMap<String, Repository> = HashMap::new();

    for repository in repositories {
        let name = repository.id.name.clone();
        let name = String::from(name.split('/').next_back().unwrap());

        if let Some(conflict) = ret.remove(&name) {
            if let Some((conflict_name, name)) =
                reduce(conflict.id.name.clone(), repository.id.name.clone())
            {
                ret.insert(conflict_name, conflict);
                ret.insert(name, repository);
            } else {
                eprintln!(
                    "Duplicated repository with name {name}: {0} and {1}.
                    {1} is ignored!",
                    conflict.root.display(),
                    repository.root.display(),
                );
                ret.insert(name, conflict);
            }
        } else {
            ret.insert(name, repository);
        }
    }

    ret
}

/// Get the map associating valid repository identifiers to the associated
/// repository present in the repo tree.
fn get_repositories(config: &Config) -> HashMap<String, Repository> {
    let repositories = load_repositories(config);

    let mut ret = reduce_repo_names(repositories.clone());

    ret.extend(repositories.iter().map(|r| (r.id.name.clone(), r.clone())));

    for (alias, repo_name) in config.command.resolve.aliases.iter() {
        if let Some(repo) = ret.get(repo_name) {
            ret.insert(alias.clone(), repo.clone());
            if let Some(repo) = ret.get(repo_name) {
                ret.insert(alias.to_string(), repo.clone());
            }
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
fn fzf_ask(repositories: &HashMap<String, Repository>) -> Option<String> {
    let fzf = which("fzf").expect(
        "fzf not found, cannot interactively ask to select repository.",
    );

    // TODO: The preview of the values is not set, therefore it is displaying
    // bad information / errors.
    let mut child = Command::new(fzf)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;

    // Provide choices on stdin
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(&repositories.keys().join("\n").into_bytes())
            .ok()?;
    }

    // Wait and read selection
    child.wait_with_output().ok().map(|output| {
        let res = String::from_utf8_lossy(&output.stdout).into_owned();

        if res.ends_with('\n') {
            String::from(res.strip_suffix('\n').unwrap())
        } else {
            res
        }
    })
}

/// Execute the `rt resolve` command.
pub fn run(config: &Config, args: ResolveArgs) -> i32 {
    if let Some(repository) = resolve(config, args.repo_id) {
        println!("{}", repository.root.display());
        0
    } else {
        2
    }
}

/// Resolve a repository identifier into a local repository.
pub fn resolve(config: &Config, repo_id: Option<String>) -> Option<Repository> {
    let repositories = get_repositories(config);

    let Some(repo_id) = repo_id.or_else(|| fzf_ask(&repositories)) else {
        eprintln!("No repository selected");
        return None;
    };

    let repo = repositories.get(&repo_id);

    if repo.is_some() {
        return repo.cloned();
    }

    let matcher = SkimMatcherV2::default();

    let mut matches: Vec<_> = repositories
        .keys()
        .filter_map(|item| {
            matcher
                .fuzzy_match(item, &repo_id)
                .map(|score| (score, item))
        })
        .collect();

    if matches.is_empty() {
        eprintln!("No match for {repo_id}");
        return None;
    }

    matches.dedup_by_key(|(_, name)| {
        repositories.get(*name).unwrap().root.to_str()
    });

    if matches.len() == 1 {
        let name = matches[0].1;
        eprintln!("Considering you meant {name}");
        Some(repositories.get(name).unwrap().clone())
    } else {
        eprintln!("Several possible match:");
        // Sort by match score
        matches.sort_by(|a, b| b.0.cmp(&a.0));

        for (_, name) in matches {
            eprintln!("- {name}");
        }

        None
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
    let repositories = get_repositories(&config);
    let matcher = SkimMatcherV2::default();
    repositories
        .keys()
        .filter_map(|item| {
            matcher.fuzzy_match(item, current).map(|_| {
                let repository = repositories.get(item).unwrap();

                CompletionCandidate::new(item)
                    .tag(
                        repository
                            .id
                            .host
                            .clone()
                            .map(|h| StyledStr::from(h.dir_name())),
                    )
                    .help(repository.id.remote_url.clone().map(StyledStr::from))
            })
        })
        .collect()
}
