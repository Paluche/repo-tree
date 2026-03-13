//! Compute the state of a jj repository. The information we want is to find
//! out if there is an action, requiring an user input that should be done.
//!
//! Typically:
//! - jj restack needed. Some mutable commits are not on top of latest
//!   immutables ones.
//! - repo has conflicts
//! - repo has unpushed commits
use std::{error::Error, path::Path};

use jj_lib::{backend::BackendResult, repo::Repo};

use super::revsets;
use crate::repository::RepoState;

/// Compute if the repository has unpushed commits. Do not take into account
/// empty commits with empty description.
fn has_unpushed_commits(repo: &dyn Repo) -> BackendResult<bool> {
    // jj log -r 'all() ~ ::remote_bookmarks() ~ (empty() & description(""))'
    revsets::has_match(
        repo,
        revsets::remote_bookmarks(repo).minus(&revsets::bare_commit()),
    )
}

fn needs_restack(
    repo_path: &Path,
    repo: &dyn Repo,
) -> Result<bool, Box<dyn Error>> {
    // Each branch must be rebased on top of a immutable reference (bookmark or
    // tag).
    // jj log -r 'mutable() ~ immutable_heads()::'
    Ok(revsets::has_match(
        repo,
        revsets::mutable(repo_path)?
            .minus(&revsets::immutable_heads(repo_path)?.parents()),
    )?)
}

fn has_conflicts(repo: &dyn Repo) -> BackendResult<bool> {
    // jj log -r 'conflicts()'
    revsets::has_match(repo, revsets::conflicts())
}

pub async fn get_repo_state(root: &Path) -> Result<RepoState, Box<dyn Error>> {
    let repo = super::load(root).await?;
    Ok(RepoState {
        unpushed_commits: has_unpushed_commits(repo.as_ref())?,
        needs_restack: needs_restack(root, repo.as_ref())?,
        has_conflicts: has_conflicts(repo.as_ref())?,
    })
}
