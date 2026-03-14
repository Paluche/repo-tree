use std::{path::Path, process::exit};

use clap::{ArgAction, Args};
use clap_complete::{PathCompleter, engine::ArgValueCompleter};
use colored::Colorize;

use crate::{
    Repository, UrlParser,
    cli::cwd_default_path,
    config::{Config, list_host_completer},
    get_repo_tree_dir,
    git::{self, GitStatus, SubmoduleStatus},
};

/// Custom git status. Concise, with all the data and without help text.
#[derive(Args, Debug, PartialEq)]
pub struct ListArgs {
    /// Filter the repositories to list the todo state list by their host. For
    /// example, "github" or "local".
    #[arg(short='H', long, add=ArgValueCompleter::new(list_host_completer))]
    host: Option<String>,
    /// Filter the repositories to by their name, within its forge. All
    /// repositories which name starts with the provided value will be
    /// listed. For example to filter only GitHub repositories from a
    /// certain organization, you could use the organization name as value
    /// for this argument, and "github" as value of the --host argument.
    #[arg(short = 'N', long = "name", action=ArgAction::Append)]
    names: Vec<String>,
}

pub fn run(args: ListArgs) -> i32 {}
