//! Display the identity to use in a repository.
use clap::Args;
use clap_complete::PathCompleter;
use clap_complete::engine::ArgValueCompleter;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::config::ResolvedIdentity;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repository::Repository;
use crate::ui::Ui;

/// Display the identity to use in a repository, and the identities it is
/// made of.
#[derive(Args)]
pub struct ShowArgs {
    /// Path to within the repository to work with.
    #[arg(short, long, add=ArgValueCompleter::new(PathCompleter::dir()))]
    repository: Option<String>,
}

/// Print the resolved identity.
fn print_identity(identity: &ResolvedIdentity) {
    println!("identities: {}", identity.sources.join(", "));
    println!("name:       {}", identity.name);
    println!("email:      {}", identity.email);
    println!(
        "signing:    {}",
        identity
            .signing
            .as_ref()
            .map_or("not managed".to_string(), ToString::to_string)
    );
    for (host, tokens) in &identity.tokens {
        println!(
            "tokens:     {host}: git {}, api {}",
            tokens.git.as_deref().unwrap_or("-"),
            tokens.api.as_deref().unwrap_or("-"),
        );
    }
}

/// Execute the `rt identity show` command.
pub async fn run(ui: &mut Ui<'_>, config: &Config, args: ShowArgs) -> i32 {
    let repository = match Repository::discover(
        ui,
        config,
        &cwd_default_path(ui, args.repository),
        ExpectedTreeStrategy::Lazy,
    )
    .await
    {
        Ok(r) => r,
        Err(err) => {
            ui.error(err);
            return 1;
        }
    };

    let Some(identity) = config.identities.resolve(&repository.id) else {
        ui.error("No identity configured");
        return 1;
    };

    print_identity(&identity);
    0
}
