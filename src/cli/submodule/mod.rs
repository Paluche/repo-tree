mod set;
// mod update;

use std::env::args;
use std::ffi::OsStr;

use clap::Parser;
use clap::Subcommand;
use clap_complete::CompletionCandidate;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;

use crate::Config;
use crate::Repository;
use crate::UrlParser;
use crate::cli::cwd_default_path;
use crate::get_repo_tree_dir;
use crate::jujutsu;

#[derive(Subcommand, Debug, PartialEq)]
pub enum SubmoduleAction {
    /// Set a submodule
    Set {
        /// Path to within the git repository to work with.
        #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
        repository: Option<String>,

        /// Path to the submodule to set.
        #[arg(add=ArgValueCompleter::new(submodule_completer))]
        submodule: String,

        /// Reference to set the submodule at. If the submodule is a jj
        /// repository then we will support revsets.
        #[arg(add=ArgValueCompleter::new(reference_completer))]
        reference: String,
    },
}

pub fn run_submodule(action: SubmoduleAction) -> i32 {
    match action {
        SubmoduleAction::Set {
            repository,
            submodule,
            reference,
        } => {
            match set::set(
                cwd_default_path(repository, true),
                submodule,
                reference,
            ) {
                Ok(()) => 0,
                Err(_) => 1,
            }
        }
    }
}

#[derive(Parser, Debug, PartialEq)]
struct CompleterParser {
    #[arg(short, long)]
    repository: Option<String>,
}

fn get_repo_path_from_args() -> Option<String> {
    let ret = args()
        // Skip until reaching the words we are completing.
        .skip_while(|s| s != "--")
        // Skip -- and the command name.
        .skip(2)
        // Search for the option specifying the repository to use.
        .skip_while(|a| a == "--repository" || a == "-r")
        // Skip the option and get the value
        .nth(2);
    eprintln!("get_repo_path_from_args {ret:?}");
    ret
}

/// This function considers it is called when the submodule argument has been
/// set in the CLI and you are editing the reference.
fn get_submodule_path_from_args() -> String {
    eprintln!("get_submodule_path_from_args()");
    let clap_complete_index: usize = std::env::var("_CLAP_COMPLETE_INDEX")
        .expect("Missing _CLAP_COMPLETE_INDEX environment variable")
        .parse()
        .expect("_CLAP_COMPLETE_INDEX is not a valid usize");
    let mut skip = false;
    let mut ret: Option<String> = None;
    for (i, arg) in args()
        // Skip until reaching the words we are completing.
        .skip_while(|s| s != "--")
        // Skip -- and the command name.
        .skip(1)
        .enumerate()
    {
        // Skip the current argument as requested.
        if skip {
            skip = false;
            continue;
        }

        // Skip the command name and the argument we are currently completing.
        if i == 0 || i == clap_complete_index {
            continue;
        }

        if arg == "--repository" || arg == "-r" {
            skip = true;
            continue;
        }

        ret = Some(arg);
    }

    ret.unwrap()
}

fn submodule_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let repo_path = cwd_default_path(get_repo_path_from_args(), false);

    let repo = match git2::Repository::discover(&repo_path) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("{} is not within a repository", repo_path.display());
            return Vec::new();
        }
    };

    repo.submodules()
        .unwrap_or(Vec::new())
        .iter()
        .filter_map(|s| {
            s.path().to_str().and_then(|res| {
                if res.starts_with(current) {
                    eprintln!("Using {res}");
                    Some(CompletionCandidate::new(res))
                } else {
                    eprintln!("Discarding {res}");
                    None
                }
            })
        })
        .collect()
}

fn reference_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };

    let repo_tree_dir = get_repo_tree_dir();
    let config = Config::default();
    let url_parser = UrlParser::new(&config);
    let main_repo_path = cwd_default_path(get_repo_path_from_args(), true);
    let main_repo = match Repository::discover(
        &repo_tree_dir,
        main_repo_path.clone(),
        &url_parser,
    ) {
        Ok(Some(r)) => r,
        Ok(None) | Err(_) => {
            eprintln!(
                "{} is not within a repository",
                main_repo_path.display()
            );
            return Vec::new();
        }
    };
    let submodule_path = main_repo.root.join(get_submodule_path_from_args());
    let sub_repo = match Repository::discover(
        &repo_tree_dir,
        submodule_path.clone(),
        &UrlParser::new(&Config::default()),
    ) {
        Ok(Some(r)) => r,
        Ok(None) | Err(_) => {
            eprintln!(
                "{} is not within a repository",
                submodule_path.display()
            );
            return Vec::new();
        }
    };

    if !sub_repo.vcs.is_jujutsu() {
        // TODO Completion for git repositories.
        return Vec::new();
    }

    let ifs = "\n";

    jujutsu::new_jj_command()
        .arg("--")
        .arg("jj")
        .arg("--repository")
        .arg(&sub_repo.root)
        .arg("log")
        .arg("-r")
        .arg(current)
        .env("_CLAP_IFS", ifs)
        .env("_CLAP_COMPLETE_INDEX", "5")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split(ifs)
                .map(CompletionCandidate::new)
                .collect()
        })
        .unwrap_or_default()
}
