//! Implementation of `rt util`, which contains sub-commands you are more likely
//! to rarely use or mostly within scripts.
use clap::Args;
use clap::CommandFactory;
use clap::Subcommand;

use crate::config::Config;

pub mod completion;

/// Actions for git repositories.
#[allow(clippy::missing_docs_in_private_items)]
#[derive(Args, Debug, PartialEq)]
pub struct UtilArgs {
    #[command(subcommand)]
    action: UtilAction,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Subcommand, Debug, PartialEq)]
enum UtilAction {
    Completion(completion::CompletionArgs),
}

/// Run the util sub-command.
pub fn run(_: &Config, args: UtilArgs) -> i32 {
    match args.action {
        UtilAction::Completion(args) => {
            completion::run(&mut crate::cli::Args::command(), args)
        }
    }
}
