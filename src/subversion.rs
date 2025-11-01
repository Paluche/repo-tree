use crate::cli::PromptBuilder;
use colored::Colorize;
use std::{path::Path, process::Command};
use which::which;

pub fn prompt(_root: &Path, info: &mut PromptBuilder) -> i32 {
    info.push_colored_string("N/A".red());
    0
}

pub fn checkout(remote_url: &str, location: &Path) -> i32 {
    Command::new(which("svn").expect("Subversion not installed"))
        .arg("checkout")
        .arg(remote_url)
        .arg(location)
        .status()
        .expect("Error executing command")
        .code()
        .unwrap()
}
