//! Dummy command to test so part of the code.

use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::forge::Forge;
use crate::repository::Repository;

/// Fetch all the repositories within the repo_tree.
#[derive(Args)]
pub struct TestArgs {
    /// Path to within the git repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
}

/// Execute `rt test` command.
pub async fn run(config: &Config, args: TestArgs) -> i32 {
    let api = Forge::GitHub.api();
    let repo_path = cwd_default_path(args.repository);

    let repository = match Repository::discover(config, repo_path.clone()) {
        Ok(r) => r,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };

    println!(
        "Repository {} is {}archived",
        &repository.id.display(&config),
        if api
            .is_archived(&repository.id)
            .await
            .expect("Unable to obtain archived status")
        {
            ""
        } else {
            "not "
        }
    );

    0
}
