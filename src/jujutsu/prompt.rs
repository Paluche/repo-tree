use itertools::Itertools;
use jj_lib::{
    backend::BackendError,
    commit::Commit,
    op_store::LocalRemoteRefTarget,
    ref_name::{RefName, WorkspaceName},
    repo::Repo,
    store::Store,
    view::View,
};

use super::load;
use crate::cli::PromptBuilder;
use colored::Colorize;
use std::{path::Path, sync::Arc};

struct CommitBookmark<'view> {
    ref_name: &'view RefName,
    // lrftp: LocalRemoteRefTarget<'view>,
    distance: i32,
}

fn commit_has_parent_internal(
    commit: &Commit,
    ref_commit: &Commit,
    mut distance: i32,
) -> Result<Option<i32>, BackendError> {
    if commit == ref_commit {
        return Ok(Some(distance));
    }

    distance += 1;

    for parent in commit.parents() {
        let parent = parent?;
        let res = commit_has_parent_internal(&parent, ref_commit, distance)?;
        if res.is_some() {
            return Ok(res);
        }
    }

    Ok(None)
}
fn commit_has_parent(
    commit: &Commit,
    ref_commit: &Commit,
) -> Result<Option<i32>, BackendError> {
    commit_has_parent_internal(commit, ref_commit, 0)
}

impl<'view> CommitBookmark<'view> {
    fn new(
        repo_store: &Arc<Store>,
        ref_name: &'view RefName,
        lrftp: LocalRemoteRefTarget<'view>,
        ref_commit: &Commit,
    ) -> Result<Option<Self>, BackendError> {
        Ok(lrftp
            .local_target
            .added_ids()
            .map(|id| repo_store.get_commit(id))
            .collect::<Result<Vec<Commit>, BackendError>>()?
            .iter()
            .filter_map(|c| commit_has_parent(c, ref_commit).transpose())
            .collect::<Result<Vec<i32>, BackendError>>()?
            .iter()
            .max()
            .map(|d| Self {
                ref_name,
                //l rftp,
                distance: *d,
            }))
    }
}

fn get_commit_bookmarks<'view>(
    repo_store: &Arc<Store>,
    repo_view: &'view View,
    commit: &Commit,
) -> Result<Vec<CommitBookmark<'view>>, BackendError> {
    repo_view
        .bookmarks()
        .filter_map(|(rf, lrft)| {
            CommitBookmark::new(repo_store, rf, lrft, commit).transpose()
        })
        .collect()
}

pub fn prompt(root: &Path, info: &mut PromptBuilder) -> i32 {
    let repo = load(root).unwrap();
    let repo_store = repo.store();
    let repo_view = repo.view();
    let workspace_name = WorkspaceName::DEFAULT;

    let commit = match repo
        .view()
        .get_wc_commit_id(workspace_name)
        .and_then(|id| repo.store().get_commit(id).ok())
    {
        None => return 1,
        Some(c) => c,
    };

    // Unneeded information in prompt. I know I am at @
    // let change_id = commit.change_id();
    // let len = repo.shortest_unique_change_id_prefix_len(change_id);
    // let change_id = change_id.to_string();

    // info.push_string(&format!(
    //     "{}{}",
    //     change_id[0..len].purple(),
    //     change_id[len..8.max(len + 1)].bright_black(),
    // ));

    let mut bookmarks = String::new();
    let mut print_other = false;
    for commit_bookmark in get_commit_bookmarks(repo_store, repo_view, &commit)
        .expect("Error retrieving bookmarks")
        .iter()
        .sorted_by_key(|cb| cb.distance)
    {
        bookmarks.push_str(&format!(
            "{}{}",
            if commit_bookmark.distance == 0 {
                "󰫍"
            } else if print_other {
                "🞍"
            } else {
                print_other = true;
                "󰫎"
            }
            .bright_blue(),
            commit_bookmark.ref_name.as_str().purple()
        ));
    }
    info.push_string(&if bookmarks.is_empty() {
        "󰫌".bright_black().to_string()
    } else {
        bookmarks
    });

    0
}
