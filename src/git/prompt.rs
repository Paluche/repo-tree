use crate::{Repository, cli::PromptBuilder, git};
use colored::Colorize;
use std::path::Path;

pub fn prompt(root: &Path, repository: Repository) -> i32 {
    let git_status = git::status(&root.to_path_buf()).unwrap();
    //  forge/repo|⛏operation|(detached) branch-1🞍branch-2🞍branch-3 tag-1🞍tag-2|◀🠟🠝|◀||
    let mut info = PromptBuilder::default();
    info.push_colored_string(repository.id.name.green());

    if !git_status.ongoing_operations.is_empty() {
        let mut operations = String::from("⛏");
        operations.push_str(&PromptBuilder::join_vec_str(
            '🞍',
            &git_status
                .ongoing_operations
                .iter()
                .map(|e| format!("{e}"))
                .collect::<Vec<String>>(),
        ));
        info.push_colored_string(operations.red());
    }

    // current branch name
    let mut branch_info = git_status.head.branch.clone();

    // All other branches at the current reference
    for (i, branch) in git_status.head.branches.iter().enumerate() {
        if i == 0 {
            branch_info.push_str(" ");
        } else {
            branch_info.push('🞍');
        }
        branch_info.push_str(branch)
    }

    // All other tags at the current reference
    for (i, branch) in git_status.head.tags.iter().enumerate() {
        if i == 0 {
            branch_info.push_str(" ");
        } else {
            branch_info.push('🞍');
        }
        branch_info.push_str(branch)
    }

    branch_info = if branch_info.len() >= 50 {
        branch_info[..50].to_string()
    } else {
        branch_info
    };

    info.push_colored_string(branch_info.yellow());

    // Upstream info
    info.push_colored_string(
        if let Some(upstream_info) = &git_status.head.upstream {
            if upstream_info.gone {
                ""
            } else if upstream_info.ahead == 0 && upstream_info.behind == 0 {
                ""
            } else if upstream_info.ahead != 0 && upstream_info.behind != 0 {
                ""
            } else if upstream_info.ahead != 0 {
                ""
            } else {
                ""
            }
        } else if git_status.head.branch == "(detached)" {
            ""
        } else {
            ""
        }
        .ansi_color(208),
    );

    let (staged, unstaged, submodules) = git_status.short_status();
    info.push_string(&format!(
        "{}{}",
        staged.as_string().green(),
        unstaged.as_string().red()
    ));

    // Submodule status
    info.push_colored_string(submodules.as_string().red());

    // stash status
    if git_status.nb_stash != 0 {
        info.push_colored_string("".white());
    }

    println!("{info}");
    0
}
