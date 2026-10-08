//! Clone a repository into the repo tree.
use std::error::Error;

use clap::Args;

use crate::config::Config;
use crate::repo_id::RepoId;
use crate::repo_tree::RepoTree;
use crate::tree_space::ForceTreeSpace;
use crate::ui::Ui;
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
    ui: &mut Ui<'_>,
    force_tree: Option<ForceTreeSpace>,
    repo_id: &RepoId,
    vcs: &VersionControlSystem,
) -> Result<(), Box<dyn Error>> {
    let location = repo_id
        .expected_tree(config, ui, None, force_tree.into())
        .await?
        .repo_location(config, repo_id)?;

    if location.exists() {
        if let Some((current_vcs, _)) = VersionControlSystem::try_new(&location)
        {
            if &current_vcs == vcs {
                ui.hint(format!(
                    "{} repository already cloned",
                    repo_id.display(config)
                ));
            } else if matches!(current_vcs, VersionControlSystem::Git)
                && matches!(vcs, VersionControlSystem::JujutsuGit)
            {
                ui.hint("Repository already cloned, initializing JJ into");
                jujutsu::init_colocate(ui, &location)?;
            } else {
                ui.hint(format!(
                    "{} repository already cloned but is a {current_vcs} \
                     repository instead of a {vcs} repository",
                    repo_id.display(config)
                ));
            }
        } else {
            ui.hint(format!(
                "Clone location {} already exists",
                location.display()
            ));
            return Ok(());
        }
    } else {
        let remote_url = &repo_id
            .remote
            .as_ref()
            .expect("Remote URL provided by the CLI")
            .url;

        vcs.get_repo(&location).clone(ui, remote_url)?;
    }

    // Refresh the cache.
    RepoTree::load(config, ui, true);

    println!("{}", location.display());
    Ok(())
}

/// Execute the `rt clone` command.
pub async fn run(config: &Config, ui: &mut Ui<'_>, args: CloneArgs) -> i32 {
    let vcs = args.vcs.unwrap_or(config.command.clone.default_vcs);

    if let Ok(repo_id) = RepoId::from_remote_url(&args.url) {
        match do_clone(config, ui, args.force_tree, &repo_id, &vcs).await {
            Ok(()) => 0,
            Err(err) => {
                ui.error(err);
                1
            }
        }
    } else {
        ui.error("Error parsing the provided URL");
        1
    }
}
