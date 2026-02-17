use clap::Subcommand;
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
    },
}

pub fn run_git(action: GitAction) -> i32 {
    match action {
        GitAction::Status { repository } => {
            status_action::status(cwd_default_path(repository))
        }
    }
}
