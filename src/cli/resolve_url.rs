//! Action to resolve the path to a repository from a remote URL.
use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Args;
use clap::builder::StyledStr;
use clap_complete::engine::ArgValueCompleter;
use clap_complete::engine::CompletionCandidate;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::Config;
use crate::load_repositories;

/// Resolve the URL of a repository into its path.
#[derive(Args, Debug, PartialEq)]
pub struct ResolveUrlArgs {
    /// Repository identifier to resolve into the actual path within the
    /// repo_tree.
    #[arg(add=ArgValueCompleter::new(resolve_completer))]
    repo_id: String,
}

/// Get the map associating remote URL to the repository present in the repo
/// tree.
fn get_repositories(config: &Config) -> BTreeMap<String, PathBuf> {
    BTreeMap::from_iter(load_repositories(config).iter().filter_map(
        |repository| {
            repository
                .id
                .remote_url
                .clone()
                .map(|u| (u, repository.root.clone()))
        },
    ))
}

/// Execute the `rt resolve-url` command.
pub fn run(config: &Config, args: ResolveUrlArgs) -> i32 {
    let repositories = get_repositories(config);
    if let Some(repo) = repositories.get(&args.repo_id) {
        println!("{}", repo.display());
        return 0;
    }

    let matcher = SkimMatcherV2::default();

    let mut matches: Vec<_> = repositories
        .keys()
        .filter_map(|item| {
            matcher
                .fuzzy_match(item, args.repo_id.as_str())
                .map(|score| (score, item))
        })
        .collect();

    if matches.is_empty() {
        eprintln!("No match for {}", args.repo_id);
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

/// Get auto-completion candidate resolution for a valid repository URL
/// argument.
fn resolve_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };
    let Ok(config) = Config::load() else {
        return vec![];
    };

    let repositories = get_repositories(&config);
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
