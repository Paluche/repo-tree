//! Action to resolve the path to a repository.
//!
//! This is typically to implement shell functions just as:
//!
//! ```bash
//! # Repository Change Directory, jump to the specified repository using its
//! # short name.
//! function rcd() {
//!    // TODO
//! }
//! ```
//!
//! ``` bash
//! # Clone a repository using jj at the correct location in the workspace.
//! function jj_clone() {
//!     // TODO
//! }
//! ```
//!
//!
use crate::{Config, Repository, UrlParser, load_workspace};
use clap::builder::StyledStr;
use clap_complete::engine::CompletionCandidate;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use itertools::Itertools;
use std::{
    collections::HashMap,
    io::Write,
    iter::zip,
    process::{Command, Stdio},
};
use which::which;

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

fn get_repositories() -> HashMap<String, Repository> {
    let (repositories, empty_dirs) =
        load_workspace(&UrlParser::new(&Config::default()));

    for empty_dir in empty_dirs {
        eprintln!("Empty directory in WORKSPACE_DIR: {}", empty_dir.display());
    }

    let mut ret = reduce_repo_names(repositories.clone());

    ret.extend(repositories.iter().map(|r| (r.id.name.clone(), r.clone())));

    ret
}

fn fzf_ask(repositories: &HashMap<String, Repository>) -> Option<String> {
    let fzf = which("fzf").expect(
        "fzf not found, cannot interactively ask to select repository.",
    );

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

pub fn resolve(query: Option<String>) -> i32 {
    let repositories = get_repositories();

    let Some(query) = query.or_else(|| fzf_ask(&repositories)) else {
        eprintln!("No repository selected");
        return 2;
    };

    if let Some(repo) = repositories.get(&query) {
        println!("{}", repo.root.display());
        return 0;
    }

    let matcher = SkimMatcherV2::default();

    let mut matches: Vec<_> = repositories
        .keys()
        .filter_map(|item| {
            matcher
                .fuzzy_match(item, query.as_str())
                .map(|score| (score, item))
        })
        .collect();

    if matches.is_empty() {
        eprintln!("No match for {query}");
        return 1;
    }

    matches.dedup_by_key(|(_, name)| {
        repositories.get(*name).unwrap().root.to_str()
    });

    if matches.len() == 1 {
        let name = matches[0].1;
        eprintln!("Considering you meant {name}");
        println!("{}", repositories.get(name).unwrap().root.display());
        0
    } else {
        eprintln!("Several possible match:");
        // Sort by match score
        matches.sort_by(|a, b| b.0.cmp(&a.0));

        for (_, name) in matches {
            eprintln!("- {name}");
        }

        2
    }
}

pub fn resolve_completer(
    current: &std::ffi::OsStr,
) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let repositories = get_repositories();
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
                            .map(|h| StyledStr::from(h.name)),
                    )
                    .help(
                        repository.id.remote_url.clone().map(StyledStr::from),
                    )
            })
        })
        .collect()
}
