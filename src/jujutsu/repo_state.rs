//! Compute the state of a jj repository. The information we want is to find
//! out if there is an action, requiring an user input that should be done.
//!
//! Typically:
//! - jj restack needed. Some mutable commits are not on top of latest
//!   immutables ones.
//! - repo has conflicts
//! - repo has unpushed commits
use std::{error::Error, path::Path};

use super::revsets;
use crate::repository::RepoState;

/// Compute if the repository has unpushed commits. Do not take into account
/// empty commits with empty description.
fn has_unpushed_commits(repo_path: &Path) -> Result<bool, Box<dyn Error>> {
    revsets::revset_has_match(
        repo_path,
        r#"::visible_heads() ~ ::(remote_bookmarks() | tags()) ~ (empty() & description(""))"#,
    )
}

fn needs_restack(repo_path: &Path) -> Result<bool, Box<dyn Error>> {
    // Each branch must be rebased on top of a immutable reference (bookmark or
    // tag).
    revsets::revset_has_match(
        repo_path,
        r#"~(::immutable_heads() | immutable_heads()::) ~ (empty() & description(""))"#,
    )
}

fn has_conflicts(repo_path: &Path) -> Result<bool, Box<dyn Error>> {
    revsets::revset_has_match(repo_path, "conflicts()")
}

#[expect(dead_code)]
pub async fn get_repo_state(
    repo_path: &Path,
) -> Result<RepoState, Box<dyn Error>> {
    Ok(RepoState {
        unpushed_commits: has_unpushed_commits(repo_path)?,
        needs_restack: needs_restack(repo_path)?,
        has_conflicts: has_conflicts(repo_path)?,
    })
}
