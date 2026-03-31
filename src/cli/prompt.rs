use std::fmt::Display;

use clap::Args;
use clap_complete::{ArgValueCompleter, PathCompleter};
use colored::{ColoredString, Colorize, control::SHOULD_COLORIZE};
use pollster::FutureExt;

use crate::{
    Config, Repository, cli::cwd_default_path, git, jujutsu,
    version_control_system::VersionControlSystem,
};

/// Generate the prompt for your shell.
#[derive(Args, Debug, PartialEq)]
pub struct PromptArgs {
    /// Path to within the repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
}

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

pub fn run(config: &Config, args: PromptArgs) -> i32 {
    let repo_path = cwd_default_path(args.repository);
    SHOULD_COLORIZE.set_override(true);

    let repo = Repository::discover(config, repo_path)
        .expect("Error loading the repository");

    if repo.is_none() {
        return 0;
    }

    let repository = repo.unwrap();
    let mut info = PromptBuilder::new(&repository);

    let ret = match repository.vcs {
        VersionControlSystem::Git => {
            git::prompt(&repository.root, false, &mut info)
        }
        VersionControlSystem::JujutsuGit => {
            let ret = git::prompt(&repository.root, true, &mut info);
            if ret != 0 {
                return ret;
            }
            jujutsu::prompt(&repository.root, &mut info).block_on()
        }
        VersionControlSystem::Jujutsu => {
            jujutsu::prompt(&repository.root, &mut info).block_on()
        }
    };

    if ret == 0 {
        println!("{info}");
    }

    ret
}
