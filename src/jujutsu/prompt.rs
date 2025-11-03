use itertools::Itertools;
use jj_lib::{
    backend::{BackendResult, CommitId},
    ref_name::WorkspaceName,
    repo::Repo,
    revset::RevsetExpression,
};

use super::load;
use crate::cli::PromptBuilder;
use colored::Colorize;
use std::path::Path;

fn commit_descendants(
    repo: &dyn Repo,
    root: &CommitId,
) -> BackendResult<Vec<CommitId>> {
    let expr = RevsetExpression::commits(vec![root.clone()]).descendants();
    let revset = expr.evaluate(repo).map_err(|e| e.into_backend_error())?;
    let ids: Vec<CommitId> = revset.iter().map(|r| r.unwrap()).collect();
    Ok(ids)
}

pub fn prompt(root: &Path, info: &mut PromptBuilder) -> i32 {
    let repo = load(root).unwrap();
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

    let Ok(descendants) = commit_descendants(repo.as_ref(), commit.id())
    else {
        return 1;
    };

    let commit_bookmarks: Vec<&str> = repo_view
        .bookmarks()
        .filter_map(|(r, lrft)| {
            if let Some(c) = lrft.local_target.as_normal()
                && descendants.iter().contains(&c)
            {
                Some(r.as_str())
            } else {
                None
            }
        })
        .collect();

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
    for commit_bookmark in commit_bookmarks {
        bookmarks.push_str(&format!(
            "{}{}",
            //if commit_bookmark.distance == 0 {
            //   "󰫍"
            //} else
            if print_other {
                "🞍"
            } else {
                print_other = true;
                "󰫎"
            }
            .bright_blue(),
            commit_bookmark.purple()
        ));
    }
    info.push_string(&if bookmarks.is_empty() {
        "󰫌".bright_black().to_string()
    } else {
        bookmarks
    });

    0
}
