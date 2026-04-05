//! Compute the state of a jj repository. The information we want is to find
//! out if there is an action, requiring an user input that should be done.
//!
//! Typically:
//! - jj restack needed. Some mutable commits are not on top of latest
//!   immutables ones.
//! - repo has conflicts
//! - repo has unpushed commits
use std::error::Error;
use std::path::Path;

use super::revsets;
use crate::repo_state::RepoState;

/// Compute if the repository has unpushed commits. Do not take into account
/// empty commits with empty description.
fn has_unpushed_commits(repo_path: &Path) -> Result<bool, Box<dyn Error>> {
    revsets::revset_has_match(
        repo_path,
        r#"::visible_heads() ~ ::(remote_bookmarks() | tags()) ~ (empty() & description(""))"#,
    )
}

/// Find out if the repository has commits that needs to be restacked / rebased.
fn needs_restack(repo_path: &Path) -> Result<bool, Box<dyn Error>> {
    // Each branch must be rebased on top of a immutable reference (bookmark or
    // tag).
    revsets::revset_has_match(
        repo_path,
        r#"~(::immutable_heads() | immutable_heads()::) ~ (empty() & description(""))"#,
    )
}

/// Find out if the repository has commits with conflicts.
fn has_conflicts(repo_path: &Path) -> Result<bool, Box<dyn Error>> {
    revsets::revset_has_match(repo_path, "conflicts()")
}

/// Get the repository state as RepoState struct.
pub async fn get_repo_state(
    repo_path: &Path,
) -> Result<RepoState, Box<dyn Error>> {
    Ok(RepoState::new(
        has_unpushed_commits(repo_path)?,
        needs_restack(repo_path)?,
        has_conflicts(repo_path)?,
    ))
}
