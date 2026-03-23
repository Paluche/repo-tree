use std::{path::Path, process::exit};

use clap::{ArgAction, Args};
use clap_complete::{PathCompleter, engine::ArgValueCompleter};
use colored::Colorize;

use crate::{
    Config, Repository,
    cli::cwd_default_path,
    git::{self, GitStatus, SubmoduleStatus},
};

/// Custom git status. Concise, with all the data and without help text.
#[derive(Args, Debug, PartialEq)]
pub struct StatusArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,

    /// Print path relative to the root of the repository and not the
    /// current working directory.
    #[arg(long, action=ArgAction::SetTrue)]
    no_relative_path: bool,
}

fn format_repo_status(
    cwd: &Path,
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
        ret.push_str(&format!(
            "┊ {}{}\n",
            prefix,
            item.display(cwd, main_repo_path, rel_path)
        ));
        if matches!(item.submodule_status, SubmoduleStatus::Submodule { .. }) {
            let mut repo_path = main_repo_path.to_path_buf();
            if let Some(rel_path) = rel_path {
                repo_path.push(rel_path);
            }
            let rel_path = item.path;
            repo_path.push(&rel_path);
            let repo_path = repo_path.to_str().unwrap();
            let status = git::status(&repo_path).unwrap();

            ret.push_str(&format_repo_status(
                cwd,
                main_repo_path,
                Some(&rel_path),
                status,
                level + 1,
            ));
        }
    }

    ret
}

pub fn run(args: StatusArgs) -> i32 {
    let repo_path = cwd_default_path(args.repository);
    let config = Config::default();
    let Some(repository) = Repository::discover(&config, repo_path.clone())
        .expect("Error loading the repository")
    else {
        eprintln!("Not within a repository");
        exit(1);
    };

    let expected_root = repository.expected_root(&config);

    if let Some(expected_root) = expected_root
        && repository.root != expected_root
    {
        eprintln!(
            "⚠️Unexpected location for the repository {}. Currently in \"{}\" \
             should be in \"{}\".",
            repository.id.name,
            repository.root.display(),
            expected_root.display(),
        );
    }

    if !repository.vcs.is_git() {
        eprintln!("Status not implemented for {}", repository.vcs);
        return 1;
    }

    println!(
        "{}",
        format_repo_status(
            if args.no_relative_path {
                &repository.root
            } else {
                repo_path.as_path()
            },
            &repository.root,
            None,
            git::status(&repository.root).expect("Error obtaining git status"),
            0
        )
    );

    0
}
