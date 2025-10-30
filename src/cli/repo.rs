//! Compute the root to the repository.
//!
use crate::VersionControlSystem;
use clap::Subcommand;

use super::get_cwd;

#[derive(Subcommand, Debug, PartialEq)]
pub enum RepoAction {
    /// Get the root and type of the repository the current working directory
    /// is in.
    Root {
        /// Get the root of the repository the parent directory of the current
        /// working directory is in.
        #[arg(long, short)]
        parent: bool,

        /// Also display repository types. The output will then have 4 words:
        /// <Root of the repository> <is_git> <is_jj> <is_subversion>
        #[arg(long)]
        print_type: bool,
    },
}

pub fn run_repo(action: RepoAction) -> i32 {
    match action {
        RepoAction::Root { parent, print_type } => {
            repo_root(parent, print_type)
        }
    }
}

fn repo_root(parent: bool, print_type: bool) -> i32 {
    let mut cwd = get_cwd();

    if parent && let Some(parent) = cwd.parent() {
        cwd = parent.to_path_buf()
    }

    if let Some((root, vcs, _)) =
        VersionControlSystem::discover_root(cwd.clone())
    {
        print!("{}", root.display());
        if print_type {
            println!(
                " {} {} {}",
                vcs.is_git(),
                vcs.is_jujutsu(),
                matches!(vcs, VersionControlSystem::Subversion),
            );
        }
        0
    } else {
        1
    }
}
