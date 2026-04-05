//! Generate Prompt line for a repository.
use clap::Args;
use clap_complete::ArgValueCompleter;
use clap_complete::PathCompleter;
use colored::control::SHOULD_COLORIZE;
use pollster::FutureExt;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::error::NoRepositoryError;
use crate::git;
use crate::jujutsu;
use crate::prompt_builder::PromptBuilder;
use crate::repository::Repository;
use crate::version_control_system::VersionControlSystem;

/// Generate the prompt for your shell.
#[derive(Args, Debug, PartialEq)]
pub struct PromptArgs {
    /// Path to within the repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
}

/// Execute `rt repo prompt` command.
pub fn run(config: &Config, args: PromptArgs) -> i32 {
    let repo_path = cwd_default_path(args.repository);
    SHOULD_COLORIZE.set_override(true);

    let repository = match Repository::discover(config, repo_path) {
        Ok(r) => r,
        Err(err) => {
            if err.downcast_ref::<NoRepositoryError>().is_some() {
                return 0;
            }
            eprintln!("{err}");
            return 1;
        }
    };

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
