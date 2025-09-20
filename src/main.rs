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
use colored::{ColoredString, Colorize};
use repo_prompt::{get_repo_info, git_status, GitStatus, SubmoduleStatus};
use std::{
    fmt::{Debug, Display},
    io,
    path::Path,
    process::exit,
};

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
        /// Repository identifier to resolve into the actual path within the
        /// workspace.
        #[arg(short, long)]
        repo_id: String,
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
        Action::Resolve { repo_id } => resolve(repo_id),
    }
}

fn join_vec_str(sep: &str, list: &[String]) -> String {
    list.iter().fold(String::new(), |mut acc, element| {
        if !acc.is_empty() {
            acc.push_str(sep);
        }
        acc.push_str(element);
        acc
    })
}

struct PromptBuilder {
    prompt: String,
    sep: String,
}

impl PromptBuilder {
    fn new() -> Self {
        Self {
            prompt: format!("{}", "".bright_purple()),
            sep: format!("{}", "|".cyan()),
        }
    }

    fn push_colored_string(&mut self, colored_string: ColoredString) {
        if !colored_string.is_empty() {
            self.prompt
                .push_str(&format!("{}{}", self.sep, colored_string));
        }
    }

    fn push_string(&mut self, string: &str) {
        if !string.is_empty() {
            self.prompt.push_str(&self.sep);
            self.prompt.push_str(string);
        }
    }
}

impl Display for PromptBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prompt)
    }
}

fn prompt(repo_path: Option<String>) {
    let repo_info = get_repo_info(repo_path)
        .inspect_err(|e| {
            eprintln!("{e}");
            exit(1);
        })
        .unwrap();
    let git_status = repo_info
        .status()
        .inspect_err(|e| {
            eprintln!("{e}");
            exit(1);
        })
        .unwrap();

    // forge/repo|(detached) branch-1🞍branch-2🞍branch-3 tag-1🞍tag-2|⛏operation|◀🠟🠝|◀||
    let mut info = PromptBuilder::new();
    info.push_colored_string(repo_info.name.green());

    if !git_status.ongoing_operations.is_empty() {
        info.push_colored_string(
            join_vec_str(
                " ",
                &git_status
                    .ongoing_operations
                    .iter()
                    .map(|e| format!("{e}"))
                    .collect::<Vec<String>>(),
            )
            .red(),
        );
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


fn resolve(repo_id: &String) {

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
