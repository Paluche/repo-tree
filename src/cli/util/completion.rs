//! Generate static autocompletion.
use std::io;

use clap::{Args, Command};
use clap_complete::{Shell, generate};

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
