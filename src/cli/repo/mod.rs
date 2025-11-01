mod root;
use clap::Subcommand;

use root::root;

#[derive(Subcommand, Debug, PartialEq)]
pub enum RepoAction {
    /// Get the root and type of the repository the working directory or its
    /// parent is into.
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
        RepoAction::Root { parent, print_type } => root(parent, print_type),
    }
}
