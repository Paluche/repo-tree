//! Clone a repository into the repo tree.
use clap::Args;

use crate::{
    Config, Host, VersionControlSystem, git, jujutsu, parse_url, repository,
};

/// Clone a repository within the repo tree.
#[derive(Args, Debug, PartialEq)]
pub struct CloneArgs {
    /// Url of the repository to clone.
    url: String,
    /// Type of version control system to use to clone the repository.
    #[arg(long, short)]
    vcs: Option<VersionControlSystem>,
}

fn do_clone(
    config: &Config,
    remote_url: String,
    host: Host,
    name: String,
    vcs: VersionControlSystem,
) -> i32 {
    let location = repository::location(&config.repo_tree_dir, &host, &name);

    if location.exists() {
        if let Some((current_vcs, _)) = VersionControlSystem::try_new(&location)
        {
            if current_vcs == vcs {
                eprintln!("{vcs} repository already cloned");
                println!("{}", location.display());
                0
            } else if matches!(current_vcs, VersionControlSystem::Git)
                && matches!(vcs, VersionControlSystem::JujutsuGit)
            {
                eprintln!("Repository already cloned, initializing JJ into");
                jujutsu::git::init_colocate(&location)
            } else {
                eprintln!(
                    "{vcs} repository already cloned but is a {current_vcs} \
                     repository"
                );
                println!("{}", location.display());
                0
            }
        } else {
            eprintln!("Clone location {} already exists", location.display());
            1
        }
    } else {
        let res = match vcs {
            VersionControlSystem::Git => git::clone(&remote_url, &location),
            VersionControlSystem::JujutsuGit => {
                let res = git::clone(&remote_url, &location);
                if res != 0 {
                    res
                } else {
                    jujutsu::git::init_colocate(&location)
                }
            }
            VersionControlSystem::Jujutsu => {
                jujutsu::git::clone(&remote_url, &location)
            }
        };
        if res == 0 {
            println!("{}", location.display());
        }
        res
    }
}

pub fn run(config: &Config, args: CloneArgs) -> i32 {
    let parsed_url = parse_url(config, &args.url);

    if let Ok((host, name)) = parsed_url {
        if let Some(host) = host {
            do_clone(
                config,
                args.url,
                host,
                name,
                args.vcs.unwrap_or(config.command.clone.vcs),
            )
        } else {
            eprintln!("Unknown host");
            1
        }
    } else {
        eprintln!("Error parsing the provided URL");
        1
    }
}
