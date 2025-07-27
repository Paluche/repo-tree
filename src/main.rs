//! Goal: Reproduce my git prompt done in shell + python.
//! The displayed information are:
//! - Repo name: Either the origin URL path.
//! - Which reference we are on
//! - ongoing operation (if there is one)
//! - ahead behind
//! - schematic git status
//! - schematic submodule status
//!
//! Custom Git status:
//! + Add remotes list
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use colored::Colorize;
use repo_prompt::{get_repo_info, git_status, GitStatus, SubmoduleStatus};
use std::{io, path::Path, process::exit};

#[derive(Parser, Debug, PartialEq)]
#[command(version, about, long_about = None)]
struct Args {
    /// Action to perform
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug, PartialEq)]
enum Action {
    Prompt {
        /// Path to within the git repository to work with.
        #[arg(short, long)]
        repository: Option<String>,
    },
    Status {
        /// Path to within the git repository to work with.
        #[arg(short, long)]
        repository: Option<String>,
    },
    Resolve {
        /// Path to within the git repository to work with.
        #[arg(short, long)]
        repository: Option<String>,
    },
    Completion {
        shell: Shell,
    },
}

fn main() {
    let args = Args::parse();

    match args.action {
        Action::Completion { shell } => {
            generate_completion(&mut Args::command(), shell);
        }
        Action::Prompt { repository } => prompt(repository),
        Action::Status { repository } => status(repository),
        Action::Resolve { .. } => panic!("Not Implemented yet"),
    }
}

fn prompt(repo_path: Option<String>) {
    let repo_info = get_repo_info(repo_path)
        .inspect_err(|e| {
            eprintln!("{e}");
            exit(1);
        })
        .unwrap();
    let git_status = repo_info.status();
    println!("{git_status:?}");
}

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

    let branch_info = &status.branch;
    let mut branch_info_line =
        format!("{} -> {}", branch_info.oid.yellow(), branch_info.head.red());
    if let Some(upstream_info) = &branch_info.upstream {
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
            let repo_path = repo_path.to_str().unwrap();
            let status = git_status(repo_path).unwrap();

            ret.push_str(&format_repo_status(
                main_repo_path,
                Some(&item.path),
                status,
                level + 1,
            ));
        }
    }

    ret
}

fn status(repo_path: Option<String>) {
    let repo_info = get_repo_info(repo_path)
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

    if let Some(expected_top_level) = expected_top_level {
        if top_level != expected_top_level {
            eprintln!(
                "⚠️Unexpected location for the repository {}. Currently in \"{}\" \
                should be in \"{}\".",
                repo_info.name,
                top_level.display(),
                expected_top_level.display(),
            );
        }
    }

    println!(
        "{}",
        format_repo_status(top_level, None, repo_info.status().unwrap(), 0)
    );
}

fn generate_completion<G: Generator + std::fmt::Debug>(
    command: &mut Command,
    generator: G,
) {
    eprintln!("Generating completion file for {generator:?}...");
    generate(
        generator,
        command,
        command.get_name().to_string(),
        &mut io::stdout(),
    );
}
