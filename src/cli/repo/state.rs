//! Print the state of the repository.
use clap::Args;
use clap_complete::{PathCompleter, engine::ArgValueCompleter};
use pollster::FutureExt;

use crate::{
    Config, Repository, UrlParser, VersionControlSystem, cli::cwd_default_path,
    get_repo_tree_dir, jujutsu,
};

/// Find out if there is something to do by the user in order to keep this
/// repository updated.
#[derive(Args, Debug, PartialEq)]
pub struct StateArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
}

pub fn run(args: StateArgs) -> i32 {
    let repo_path = cwd_default_path(args.repository);
    let repo_tree_dir = get_repo_tree_dir();
    let Some(repository) = Repository::discover(
        &repo_tree_dir,
        repo_path.clone(),
        &UrlParser::new(&Config::default()),
    )
    .expect("Error loading the repository") else {
        eprintln!("Not within a repository");
        return 1;
    };

    let repo_state = match repository.vcs {
        VersionControlSystem::Jujutsu | VersionControlSystem::JujutsuGit => {
            jujutsu::get_repo_state(&repository.root)
        }
        vcs => {
            eprintln!(
                "Repository state not implemented for {vcs} Version Control \
                 Systems"
            );
            return 1;
        }
    }
    .block_on()
    .unwrap();

    println!("{repo_state}");
    0
}
