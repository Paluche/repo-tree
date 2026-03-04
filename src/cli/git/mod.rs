use clap::{ArgAction, Subcommand};
use clap_complete::{PathCompleter, engine::ArgValueCompleter};

use crate::cli::cwd_default_path;

mod status_action;

#[derive(Subcommand, Debug, PartialEq)]
pub enum GitAction {
    /// Custom git status. Concise, with all the data and without help text.
    Status {
        /// Path to within the git repository to work with.
        #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
        repository: Option<String>,

        /// Print path relative to the root of the repository and not the
        /// current working directory.
        #[arg(long, action=ArgAction::SetTrue)]
        no_relative_path: bool,
    },
}

pub fn run_git(action: GitAction) -> i32 {
    match action {
        GitAction::Status {
            repository,
            no_relative_path,
        } => status_action::status(
            cwd_default_path(repository),
            no_relative_path,
        ),
    }
}
