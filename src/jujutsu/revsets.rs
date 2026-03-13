//! Definition of common revsets into functions.

use std::{error::Error, path::Path, process::Command};

pub fn revset_has_match(
    repo_path: &Path,
    revset: &str,
) -> Result<bool, Box<dyn Error>> {
    Ok(String::from_utf8(
        Command::new("jj")
            .arg("--ignore-working-copy")
            .arg("--repository")
            .arg(repo_path)
            .arg("log")
            .arg("-r")
            .arg(revset)
            .arg("--no-graph")
            .arg("--template")
            .arg("commit_id ++ \"\n\"")
            .output()?
            .stdout,
    )?
    .split("\n")
    .any(|line| !line.is_empty()))
}
