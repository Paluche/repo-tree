//! Action to resolve the path to a repository from a remote URL.
use std::{
    collections::BTreeMap,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use clap::{Args, builder::StyledStr};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use itertools::Itertools;
use which::which;

use crate::{Config, load_repositories};

/// Resolve the name of a repository into its path.
#[derive(Args, Debug, PartialEq)]
pub struct ResolveUrlArgs {
    /// Repository identifier to resolve into the actual path within the
    /// repo_tree.
    #[arg(add=ArgValueCompleter::new(resolve_completer))]
    repo_id: Option<String>,
}

fn get_repositories() -> BTreeMap<String, PathBuf> {
    BTreeMap::from_iter(
        load_repositories(&Config::default())
            .iter()
            .filter_map(|repository| {
                repository
                    .id
                    .remote_url
                    .clone()
                    .map(|u| (u, repository.root.clone()))
            }),
    )
}

fn fzf_ask(repositories: &BTreeMap<String, PathBuf>) -> Option<String> {
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

pub fn run(args: ResolveUrlArgs) -> i32 {
    let repositories = get_repositories();

    let Some(query) = args.repo_id.or_else(|| fzf_ask(&repositories)) else {
        eprintln!("No repository selected");
        return 2;
    };

    if let Some(repo) = repositories.get(&query) {
        println!("{}", repo.display());
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

    matches.dedup_by_key(|(_, name)| repositories.get(*name).unwrap().to_str());

    if matches.len() == 1 {
        let name = matches[0].1;
        eprintln!("Considering you meant {name}");
        println!("{}", repositories.get(name).unwrap().display());
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

fn resolve_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let repositories = get_repositories();
    let matcher = SkimMatcherV2::default();
    repositories
        .iter()
        .filter_map(|(item, path)| {
            matcher.fuzzy_match(item, current).map(|_| {
                CompletionCandidate::new(item)
                    .tag(Some(StyledStr::from(format!("{}", path.display()))))
            })
        })
        .collect()
}
