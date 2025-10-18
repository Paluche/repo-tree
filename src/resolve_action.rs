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
use crate::{Repository, load_workspace};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use std::{collections::HashMap, iter::zip};

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
    repositories: &Vec<Repository>,
) -> HashMap<String, &Repository> {
    let mut ret: HashMap<String, &Repository> = HashMap::new();

    for repository in repositories {
        let name = repository.name.clone();
        let name = String::from(name.split('/').next_back().unwrap());

        if let Some(conflict) = ret.remove(&name) {
            if let Some((conflict_name, name)) =
                reduce(conflict.name.clone(), repository.name.clone())
            {
                ret.insert(conflict_name, conflict);
                ret.insert(name, repository);
            } else {
                eprintln!(
                    "Duplicated repository with name {name}: {0} and {1}.
                    {1} is ignored!",
                    conflict.root().display(),
                    repository.root().display(),
                );
                ret.insert(name, conflict);
            }
        } else {
            ret.insert(name, repository);
        }
    }

    ret
}

pub fn resolve(query: String) -> i32 {
    let repositories = load_workspace();
    //let full_name_repositories: HashMap<String, &Repository>= HashMap::from_iter(
    //    repositories.iter().map(|r| (r.name, r))
    //);
    let short_name_repositories = reduce_repo_names(&repositories);
    let matcher = SkimMatcherV2::default();

    let mut matches: Vec<_> = short_name_repositories
        .keys()
        .filter_map(|item| {
            matcher
                .fuzzy_match(item, query.as_str())
                .map(|score| (score, item))
        })
        .collect();

    if matches.is_empty() {
        eprintln!("No match for {query}");
        1
    } else if matches.len() == 1 {
        let (score, name) = matches[0];
        if score < 100 {
            eprintln!("Considering you meant {name}");
        }
        println!(
            "{}",
            short_name_repositories
                .get(matches[0].1)
                .unwrap()
                .root
                .display()
        );
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
