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
use git2::Repository;
use repo_prompt::{
    get_git_dir, get_last_fetched, get_stashed, git_status_porcelain,
    parse_repo_url,
};
use std::{env, io, path::Path, process::exit};

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
    let git_status = git_status_porcelain(
        load_repository(repo_path)
            .workdir()
            .unwrap()
            .to_str()
            .unwrap(),
    );
    println!("{git_status:?}");
}

fn repo_status(repo_path: &str, level: usize) -> String {
    let mut info = Vec::<String>::new();
    let mut items = Vec::<String>::new();
    let git_dir = get_git_dir(repo_path).unwrap();

    if let Some(last_fetched) = get_last_fetched(&git_dir) {
        info.push(format!(
            "{} {}",
            "Last Fetched".green(),
            last_fetched.format("%c").to_string().green()
        ));
    }

    let stashed = get_stashed(&git_dir);

    if stashed != 0 {
        info.push(format!(
            "{} {}",
            stashed.to_string().bright_yellow(),
            (if stashed == 1 {
                "stash pending"
            } else {
                "stashes pending"
            })
            .bright_yellow()
        ));
    }
    let git_status = git_status_porcelain(repo_path).unwrap();
    let branch_info = &git_status.branch;
    let mut branch_info_line =
        format!("{} -> {}", branch_info.oid.yellow(), branch_info.head.red());
    if let Some(upstream_info) = &branch_info.upstream {
        branch_info_line.push_str(&format!(" {upstream_info}"));
    }

    for item in git_status.status {
        items.push(format!("{item}"));
    }

    fn get_prefix(level: usize, prefix: &str) -> String {
        let mut ret = String::new();
        for _ in 1..level {
            ret.push_str("        ");
        }
        if level != 0 {
            ret.push_str(&format!("{prefix:2} "));
        }
        ret
    }

    let mut ret: String = info
        .iter()
        .map(|element| format!("{}{}\n", get_prefix(level, "│"), element))
        .collect();

    if !items.is_empty() {
        let last = items.pop().unwrap();

        ret.push_str(
            items
                .iter()
                .map(|element| {
                    format!("{}{}\n", get_prefix(level, "├─"), element)
                })
                .collect::<String>()
                .as_str(),
        );
        ret.push_str(&format!("{}{}", get_prefix(level, "└─"), last));
    }

    ret
}

fn status(repo_path: Option<String>) {
    let repo = load_repository(repo_path);
    let (forge, repo_path) = parse_repo_url(&repo).unwrap();

    let work_dir = env::var("WORK_DIR").unwrap();
    let mut expected_path = Path::new(&work_dir).to_path_buf();
    expected_path.push(forge);
    expected_path.push(&repo_path);
    let expected_path = expected_path.as_path();
    let current_repo_path = repo.workdir().unwrap();

    if current_repo_path != expected_path {
        eprintln!(
            "⚠️Unexpected location for the repository {}. Currently in \"{}\" \
            should be in \"{}\".",
            repo_path,
            current_repo_path.display(),
            expected_path.display(),
        );
    }

    let current_repo_path = current_repo_path.to_str().unwrap();
    println!("{}", repo_status(current_repo_path, 0));
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

fn load_repository(repo_path: Option<String>) -> Repository {
    let repo_path = repo_path
        .unwrap_or(String::from(env::current_dir().unwrap().to_str().unwrap()));
    Repository::discover(repo_path)
        .inspect_err(|e| {
            println!("{}", e.message());
            exit(1);
        })
        .unwrap()
}
