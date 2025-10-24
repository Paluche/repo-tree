use crate::{Repository, UrlParser, get_work_dir, git};
use colored::{ColoredString, Colorize, control::SHOULD_COLORIZE};
use std::{fmt::Display, path::PathBuf};

struct PromptBuilder {
    prompt: String,
    sep: String,
}

impl PromptBuilder {
    fn new() -> Self {
        Self {
            prompt: format!("{}{}", "┣━┫".cyan(), "".bright_purple()),
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

fn join_vec_str(sep: char, list: &[String]) -> String {
    list.iter().fold(String::new(), |mut acc, element| {
        if !acc.is_empty() {
            acc.push(sep);
        }
        acc.push_str(element);
        acc
    })
}

fn git_prompt(repo: Repository) -> i32 {
    let git_status = git::status(&repo.root).unwrap();
    //  forge/repo|⛏operation|(detached) branch-1🞍branch-2🞍branch-3 tag-1🞍tag-2|◀🠟🠝|◀||
    SHOULD_COLORIZE.set_override(true);
    let mut info = PromptBuilder::new();
    info.push_colored_string(repo.name.green());

    if !git_status.ongoing_operations.is_empty() {
        let mut operations = String::from("⛏");
        operations.push_str(&join_vec_str(
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

pub fn prompt(repo_path: PathBuf) -> i32 {
    let repo = Repository::discover(
        &get_work_dir(),
        repo_path,
        &UrlParser::default(),
    )
    .expect("Error loading the repository");

    if let Some(repo) = repo {
        if repo.vcs.is_git() {
            return git_prompt(repo);
        }
        eprintln!("Prompt not yet implemented for {}", repo.vcs);
    }

    0
}
