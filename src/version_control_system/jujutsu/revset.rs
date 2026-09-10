//! Function to list commit information based on a revset.

use std::error::Error;
use std::path::Path;

use super::command::JujutsuCommand;

/// Option to specify in which order to obtain the list of commit.
#[derive(Default)]
pub enum RevSetOrder {
    /// Children commit first (start from the HEAD of your branches).
    #[default]
    ChildrenFirst,
    /// Start by the parent commits.
    ParentFirst,
}

/// Execute a revset and get information on the matching commits following the
/// provided template.
fn run_revset(
    repo_path: &Path,
    revset: &str,
    template: &str,
    order: RevSetOrder,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut command = JujutsuCommand::new()?;
    command
        .ignore_working_copy()
        .repository(repo_path)
        .arg("log")
        .arg("--revision")
        .arg(revset)
        .arg("--no-graph")
        .arg("--template")
        .arg(format!(r#"{template} ++ "\n""#));

    if matches!(order, RevSetOrder::ParentFirst) {
        command.arg("--reversed");
    }

    command.output_lines()
}

/// List commits ID matching the provided revset.
pub fn list_commits_id(
    repo_path: &Path,
    revset: &str,
    order: RevSetOrder,
) -> Result<Vec<String>, Box<dyn Error>> {
    run_revset(repo_path, revset, "commit_id", order)
}

/// Find out if any commit matches the provided revset.
pub fn revset_has_match(
    repo_path: &Path,
    revset: &str,
) -> Result<bool, Box<dyn Error>> {
    Ok(!list_commits_id(repo_path, revset, RevSetOrder::default())?.is_empty())
}

/// List the local bookmarks which commit there are attached to matches the
/// provided revset.
pub fn list_bookmarks(
    repo_path: &Path,
    revset: &str,
    order: RevSetOrder,
) -> Result<Vec<String>, Box<dyn Error>> {
    // Using filter in the template. Keep remote branches only when there is a
    // no local Bookmark tracking it.
    run_revset(
        repo_path,
        revset,
        r#"bookmarks
        .filter(|b| !b.remote() || !b.tracking_present())
        .map(|b| b.name()).join("\n")"#,
        order,
    )
}

/// List the name of the tags which commit there are attached to matches the
/// provided revset.
pub fn list_tags(
    repo_path: &Path,
    revset: &str,
    order: RevSetOrder,
) -> Result<Vec<String>, Box<dyn Error>> {
    run_revset(
        repo_path,
        revset,
        r#"tags.map(|b| b.name()).join("\n")"#,
        order,
    )
}
