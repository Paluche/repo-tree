use clap::{Args, CommandFactory, Subcommand};

mod completion;

/// Actions for git repositories.
#[derive(Args, Debug, PartialEq)]
pub struct UtilArgs {
    #[command(subcommand)]
    action: UtilAction,
}

#[derive(Subcommand, Debug, PartialEq)]
enum UtilAction {
    Completion(completion::CompletionArgs),
}

pub fn run(args: UtilArgs) -> i32 {
    match args.action {
        UtilAction::Completion(args) => {
            completion::run(&mut crate::cli::Args::command(), args)
        }
    }
}
