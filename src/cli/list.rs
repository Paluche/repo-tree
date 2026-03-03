use clap_complete::engine::CompletionCandidate;

use crate::{Config, UrlParser, get_repo_tree_dir, load_repo_tree};

pub fn list(host: Option<String>) -> i32 {
    let repositories = load_repo_tree(
        &get_repo_tree_dir(),
        &UrlParser::new(&Config::default()),
    )
    .0;

    for repository in repositories {
        if let Some(host) = &host {
            if let Some(repo_host) = repository.id.host {
                if &repo_host.name != host {
                    continue;
                }
            } else if host != "local" {
                continue;
            }
        }
        println!("{}", repository.root.display());
    }
    0
}

pub fn list_host_completer(
    current: &std::ffi::OsStr,
) -> Vec<CompletionCandidate> {
    Config::default().host_completer(current)
}
