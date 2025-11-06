use crate::{
    Config, Repository, UrlParser, get_workspace_dir, git, jujutsu,
    version_control_system::VersionControlSystem,
};
use colored::{ColoredString, Colorize, control::SHOULD_COLORIZE};
use std::{fmt::Display, path::PathBuf};

pub struct PromptBuilder {
    prompt: String,
    sep: String,
}

impl PromptBuilder {
    fn new(repository: &Repository) -> Self {
        let sep = format!("{}", "|".cyan());
        Self {
            prompt: format!(
                "{}{}{sep}{}{sep}{}",
                "┣━┫".cyan(),
                repository.vcs.short_display(),
                repository
                    .id
                    .host
                    .clone()
                    .map_or("".red().to_string(), |h| h.repr),
                repository.id.name.green()
            ),
            sep,
        }
    }
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

impl Display for PromptBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prompt)
    }
}

pub fn prompt(repo_path: PathBuf) -> i32 {
    SHOULD_COLORIZE.set_override(true);

    let repo = Repository::discover(
        &get_workspace_dir(),
        repo_path,
        &UrlParser::new(&Config::default()),
    )
    .expect("Error loading the repository");

    if repo.is_none() {
        return 0;
    }

    let (root, repository) = repo.unwrap();
    let mut info = PromptBuilder::new(&repository);

    let ret = match repository.vcs {
        VersionControlSystem::Git => git::prompt(&root, &mut info),
        VersionControlSystem::JujutsuGit => {
            let ret = git::prompt(&root, &mut info);
            if ret != 0 {
                return ret;
            }
            jujutsu::prompt(&root, &mut info)
        }
        VersionControlSystem::Jujutsu => jujutsu::prompt(&root, &mut info),
    };

    if ret == 0 {
        println!("{info}");
    }

    ret
}
