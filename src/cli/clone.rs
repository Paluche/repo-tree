//! Clone a repository into the repo tree.
use clap::Args;

use crate::Config;
use crate::RepoId;
use crate::VersionControlSystem;
use crate::git;
use crate::jujutsu;

/// Clone a repository within the repo tree.
#[derive(Args, Debug, PartialEq)]
pub struct CloneArgs {
    /// Url of the repository to clone.
    url: String,
    /// Type of version control system to use to clone the repository.
    #[arg(long, short)]
    vcs: Option<VersionControlSystem>,
}

/// Do the cloning of the repository.
fn do_clone(
    config: &Config,
    repo_id: &RepoId,
    vcs: &VersionControlSystem,
) -> i32 {
    let location = match repo_id.location(config) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

    if location.exists() {
        if let Some((current_vcs, _)) = VersionControlSystem::try_new(&location)
        {
            if &current_vcs == vcs {
                eprintln!("{vcs} repository already cloned");
            } else if matches!(current_vcs, VersionControlSystem::Git)
                && matches!(vcs, VersionControlSystem::JujutsuGit)
            {
                eprintln!("Repository already cloned, initializing JJ into");
                let res = jujutsu::git::init_colocate(&location);
                if res != 0 {
                    return res;
                }
            } else {
                eprintln!(
                    "{vcs} repository already cloned but is a {current_vcs} \
                     repository"
                );
            }
            println!("{}", location.display());
            0
        } else {
            eprintln!("Clone location {} already exists", location.display());
            1
        }
    } else if let Some(remote_url) = &repo_id.remote_url {
        let res = match vcs {
            VersionControlSystem::Git => git::clone(remote_url, &location),
            VersionControlSystem::JujutsuGit => {
                jujutsu::git::clone(remote_url, &location, true)
            }
            VersionControlSystem::Jujutsu => {
                jujutsu::git::clone(remote_url, &location, false)
            }
        };
        if res == 0 {
            println!("{}", location.display());
        }
        res
    } else {
        panic!(
            "The RepoId should have a remote URL, since it has been provided \
             by the CLI"
        );
    }
}

/// Execute the `rt clone` command.
pub fn run(config: &Config, args: CloneArgs) -> i32 {
    let vcs = args.vcs.unwrap_or(config.command.clone.vcs);

    if let Ok(repo_id) = RepoId::parse_url(config, &args.url) {
        do_clone(config, &repo_id, &vcs)
    } else {
        eprintln!("Error parsing the provided URL");
        1
    }
}
