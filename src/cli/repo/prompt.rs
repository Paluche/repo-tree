//! Generate Prompt line for a repository.
use clap::Args;
use clap_complete::{ArgValueCompleter, PathCompleter};
use colored::control::SHOULD_COLORIZE;
use pollster::FutureExt;

use crate::{
    Config, PromptBuilder, Repository, cli::cwd_default_path, git, jujutsu,
    version_control_system::VersionControlSystem,
};

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
