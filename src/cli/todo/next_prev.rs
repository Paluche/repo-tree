//! Obtain the path to the next or previous repository where there is something
//! to be done by the user.
use clap::ArgAction;
use clap::Args;
use clap_complete::engine::ArgValueCompleter;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;
use globset::Glob;

use crate::cli::cwd_default_path;
use crate::config::Config;
use crate::config::list_host_completer;
use crate::error::NoRepositoryError;
use crate::error::NotImplementedError;
use crate::repo_id::ExpectedTreeStrategy;
use crate::repository::Repository;
use crate::tree::RepoTree;
use crate::utils::into_iter_from;

/// Go to the next or previous repository where you have to do something to keep
/// it up-to-date.
#[derive(Args)]
pub struct NextPrevArgs {
    /// Filter the repositories to list by their host. For example, "github" or
    /// "local". You can specify glob patterns. Can be specified multiple times
    /// as an union filter.
    #[arg(
        short='H', long="host", action=ArgAction::Append,
        add=ArgValueCompleter::new(list_host_completer)
        )
    ]
    hosts: Vec<Glob>,
    /// Filter the repositories to by their name. You can specify glob
    /// patterns. For example to filter only GitHub repositories from a
    /// certain organization (e.g. 'owner'), you could use the 'owner/*' as
    /// value for this argument, and "github" as value of the --host
    /// argument. Can be specified multiple times as an union filter.
    #[arg(short = 'N', long = "name", action=ArgAction::Append)]
    names: Vec<Glob>,
    /// Force recreating the cache.
    #[arg(short = 'R', long, global = true)]
    refresh_cache: bool,
}

/// Execute the `rt todo next` or `rt todo prev` command.
pub async fn run(config: &Config, args: NextPrevArgs, reverse: bool) -> i32 {
    let repo_path = cwd_default_path(None);
    let current_repository = match Repository::discover(
        config,
        &repo_path,
        ExpectedTreeStrategy::Lazy,
    )
    .await
    {
        Ok(r) => Some(r),
        Err(err) => {
            if err.downcast_ref::<NoRepositoryError>().is_none() {
                eprintln!("Error: {err}");
                return 1;
            }
            None
        }
    };

    let repo_tree = RepoTree::load(config, args.refresh_cache);

    // Skip the current repository.
    for repository in into_iter_from(
        repo_tree.filtered(config, &args.hosts, &args.names),
        &current_repository,
        reverse,
    ) {
        if repository.id.is_local() {
            continue;
        }
        eprint!("\r{}{}", Clear(ClearType::CurrentLine), repository.id.name);
        if let Some(repo_state) = match &repository.state().await {
            Ok(v) => Some(v),
            Err(err) => {
                if err.downcast_ref::<NotImplementedError>().is_some() {
                    None
                } else {
                    eprintln!("{err}");

                    return 1;
                }
            }
        } {
            if repo_state.is_ok() {
                continue;
            }
            let remote_host_repr =
                if let Some(r) = repository.id.remote_host_repr(config) {
                    format!("{r} ")
                } else {
                    "".to_string()
                };
            eprintln!(
                "\r{}{}{:20} {}",
                Clear(ClearType::CurrentLine),
                remote_host_repr,
                repository.id.name,
                repo_state
            );
            println!("{}", repository.root.display());
            return 0;
        }
    }

    eprint!("\r{}", Clear(ClearType::CurrentLine));
    eprintln!("Nothing to do.");
    0
}
