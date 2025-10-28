//! Module for retrieving JuJutsu information.
use crate::{cli::PromptBuilder, git};
use colored::Colorize;
use git2::Repository;
use std::{
    error::Error,
    path::{Path, PathBuf},
};

pub fn get_remote_url<P: AsRef<Path>>(
    repo_path: P,
) -> Result<Option<String>, Box<dyn Error>> {
    let mut git_dir = PathBuf::new();
    git_dir.push(&repo_path);
    git_dir.push(".jj");
    git_dir.push("repo");
    git_dir.push("store");
    git_dir.push("git");
    let repo = Repository::open(git_dir)?;

    Ok(git::get_remote_url_repo(&repo)?)
}

pub fn prompt(_root: &Path, info: &mut PromptBuilder) -> i32 {
    info.push_colored_string("N/A".red());
    0
}
