//! Generate static autocompletion.
use std::io;

use clap::Args;
use clap::Command;
use clap_complete::Shell;
use clap_complete::generate;

/// Generate static completion file.
#[derive(Args, Debug, PartialEq)]
pub struct CompletionArgs {
    /// Shell for which to generate the static auto-completion file.
    shell: Shell,
}

/// Execute the `util completion` command.
pub fn run(command: &mut Command, args: CompletionArgs) -> i32 {
    let generator = args.shell;
    eprintln!("Generating completion file for {generator:?}...");
    generate(
        generator,
        command,
        command.get_name().to_string(),
        &mut io::stdout(),
    );

    0
}
