use crate::{
    UrlParser,
    git::{GitStatus, SubmoduleStatus, get_repo_info, git_status},
};
use colored::Colorize;
use std::{path::Path, process::exit};

fn format_repo_status(
    main_repo_path: &Path,
    rel_path: Option<&str>,
    status: GitStatus,
    level: usize,
) -> String {
    let mut ret = String::new();
    let prefix = (0..level).map(|_| "        ┊ ").collect::<String>();

    if let Some(last_fetched) = status.last_fetched {
        ret.push_str(&format!(
            "┊ {}{} {}\n",
            prefix,
            "Last Fetched".green(),
            last_fetched.format("%c").to_string().green()
        ));
    }

    let head_info = &status.head;
    let mut branch_info_line =
        format!("{} -> {}", head_info.oid.yellow(), head_info.branch.red());
    if let Some(upstream_info) = &head_info.upstream {
        branch_info_line.push_str(&format!(" {upstream_info}"));
    }
    ret.push_str(&format!("┊ {prefix}{branch_info_line}\n"));

    if status.nb_stash != 0 {
        ret.push_str(&format!(
            "┊ {}{} {}\n",
            prefix,
            status.nb_stash.to_string().bright_yellow(),
            (if status.nb_stash == 1 {
                "stash pending"
            } else {
                "stashes pending"
            })
            .bright_yellow()
        ));
    }

    if !status.ongoing_operations.is_empty() {
        ret.push_str(&format!(
            "┊ {}{} {}\n",
            prefix,
            status
                .ongoing_operations
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    let mut ret = String::new();
                    if i != 0 {
                        ret.push_str(", ");
                    }
                    ret.push_str(&format!("{value}"));
                    ret
                })
                .collect::<String>()
                .red(),
            "ongoing".red()
        ));
    };

    for item in status.status {
        ret.push_str(&format!("┊ {}{}\n", prefix, item.display(rel_path)));
        if matches!(item.submodule_status, SubmoduleStatus::Submodule { .. }) {
            let mut repo_path = main_repo_path.to_path_buf();
            if let Some(rel_path) = rel_path {
                repo_path.push(rel_path);
            }
            let rel_path = item.path;
            repo_path.push(&rel_path);
            let repo_path = repo_path.to_str().unwrap();
            let status = git_status(&repo_path).unwrap();

            ret.push_str(&format_repo_status(
                main_repo_path,
                Some(&rel_path),
                status,
                level + 1,
            ));
        }
    }

    ret
}

pub fn status(repo_path: String) {
    let repo_info = get_repo_info(repo_path, &UrlParser::default())
        .inspect_err(|e| {
            eprintln!("{e}");
            exit(1);
        })
        .unwrap();
    let top_level = repo_info.top_level();

    if top_level.is_none() {
        eprintln!("Bare git repository");
        exit(1);
    }

    let top_level = top_level.unwrap();
    let expected_top_level = repo_info.expected_top_level();

    if let Some(expected_top_level) = expected_top_level
        && top_level != expected_top_level
    {
        eprintln!(
            "⚠️Unexpected location for the repository {}. Currently in \"{}\" \
                should be in \"{}\".",
            repo_info.name,
            top_level.display(),
            expected_top_level.display(),
        );
    }

    println!(
        "{}",
        format_repo_status(top_level, None, repo_info.status().unwrap(), 0)
    );
}
