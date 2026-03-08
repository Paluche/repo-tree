use std::io;

use clap::{Args, Command};
use clap_complete::{Shell, generate};

/// Generate static completion file.
#[derive(Args, Debug, PartialEq)]
pub struct CompletionArgs {
    shell: Shell,
}

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
