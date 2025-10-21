use crate::{Repository, UrlParser, get_work_dir, git};
use colored::{ColoredString, Colorize, control::SHOULD_COLORIZE};
use std::{fmt::Display, path::PathBuf};

pub struct PromptBuilder {
    prompt: String,
    sep: String,
}

impl PromptBuilder {
    pub fn push_colored_string(&mut self, colored_string: ColoredString) {
        if !colored_string.is_empty() {
            self.prompt
                .push_str(&format!("{}{}", self.sep, colored_string));
        }
    }

    pub fn push_string(&mut self, string: &str) {
        if !string.is_empty() {
            self.prompt.push_str(&self.sep);
            self.prompt.push_str(string);
        }
    }

    pub fn join_vec_str(sep: char, list: &[String]) -> String {
        list.iter().fold(String::new(), |mut acc, element| {
            if !acc.is_empty() {
                acc.push(sep);
            }
            acc.push_str(element);
            acc
        })
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self {
            prompt: format!("{}{}", "┣━┫".cyan(), "".bright_purple()),
            sep: format!("{}", "|".cyan()),
        }
    }
}

impl Display for PromptBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prompt)
    }
}

pub fn prompt(repo_path: PathBuf) -> i32 {
    let repo = Repository::discover(
        &get_work_dir(),
        repo_path,
        &UrlParser::default(),
    )
    .expect("Error loading the repository");

    SHOULD_COLORIZE.set_override(true);

    if let Some((root, repository)) = repo {
        if repository.vcs.is_git() {
            return git::prompt(&root, repository);
        }
        eprintln!("Prompt not yet implemented for {}", repository.vcs);
    }

    0
}
