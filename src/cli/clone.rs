//! Clone a repository into the repo tree.
use clap::Args;

use crate::config::Config;
use crate::git;
use crate::jujutsu;
use crate::repo_id::RepoId;
use crate::repository::Repositories;
use crate::version_control_system::VersionControlSystem;

/// Clone a repository within the repo tree.
#[derive(Args)]
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
                eprintln!(
                    "{} repository already cloned",
                    repo_id.display(config)
                );
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
                    "{} repository already cloned but is a {current_vcs} \
                     repository instead of a {vcs} repository",
                    repo_id.display(config)
                );
            }
        } else {
            eprintln!("Clone location {} already exists", location.display());
            return 1;
        }
    } else {
        let remote_url = &repo_id
            .remote
            .url()
            .expect("Remote URL provided by the CLI");

        let res = match vcs {
            VersionControlSystem::Git => git::clone(remote_url, &location),
            VersionControlSystem::JujutsuGit => {
                println!("{remote_url}");
                jujutsu::git::clone(remote_url, &location, true)
            }
            VersionControlSystem::Jujutsu => {
                jujutsu::git::clone(remote_url, &location, false)
            }
        };

        if res != 0 {
            return res;
        }
    }

    // Refresh the cache.
    Repositories::load(config, true);

    println!("{}", location.display());
    0
}

/// Execute the `rt clone` command.
pub fn run(config: &Config, args: CloneArgs) -> i32 {
    let vcs = args.vcs.unwrap_or(config.command.clone.default_vcs);

    if let Ok(repo_id) = RepoId::from_remote_url(&args.url) {
        do_clone(config, &repo_id, &vcs)
    } else {
        eprintln!("Error parsing the provided URL");
        1
    }
}
