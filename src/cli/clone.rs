//! Clone a repository into the repo tree.
use std::error::Error;

use clap::Args;

use super::ForceTreeSpace;
use super::force_tree_into_strategy;
use crate::config::Config;
use crate::repo_id::RepoId;
use crate::tree::RepoTree;
use crate::version_control_system::VersionControlSystem;
use crate::version_control_system::jujutsu;

/// Clone a repository within the repo tree.
#[derive(Args)]
pub struct CloneArgs {
    /// Url of the repository to clone.
    url: String,
    /// Type of version control system to use to clone the repository.
    #[arg(long, short)]
    vcs: Option<VersionControlSystem>,
    /// If a doubt subsist as if the repository to clone is an archive or not.
    #[arg(long, short)]
    force_tree: Option<ForceTreeSpace>,
}

/// Do the cloning of the repository.
async fn do_clone(
    config: &Config,
    force_tree: Option<ForceTreeSpace>,
    repo_id: &RepoId,
    vcs: &VersionControlSystem,
) -> Result<i32, Box<dyn Error>> {
    let location = repo_id
        .expected_tree(config, None, force_tree_into_strategy(force_tree))
        .await?
        .repo_location(config, repo_id)?;

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
                let res = jujutsu::init_colocate(&location);
                if res != 0 {
                    return Ok(res);
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
            return Ok(1);
        }
    } else {
        let remote_url = &repo_id
            .remote
            .as_ref()
            .expect("Remote URL provided by the CLI")
            .url;

        let res = vcs.get_repo(&location).clone(remote_url);

        if res != 0 {
            return Ok(res);
        }
    }

    // Refresh the cache.
    RepoTree::load(config, true);

    println!("{}", location.display());
    Ok(0)
}

/// Execute the `rt clone` command.
pub async fn run(config: &Config, args: CloneArgs) -> i32 {
    let vcs = args.vcs.unwrap_or(config.command.clone.default_vcs);

    if let Ok(repo_id) = RepoId::from_remote_url(&args.url) {
        match do_clone(config, args.force_tree, &repo_id, &vcs).await {
            Ok(c) => c,
            Err(err) => {
                eprintln!("{err}");
                1
            }
        }
    } else {
        eprintln!("Error parsing the provided URL");
        1
    }
}
