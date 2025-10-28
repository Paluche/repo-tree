use crate::cli::PromptBuilder;
use colored::Colorize;
use std::path::Path;

pub fn prompt(_root: &Path, info: &mut PromptBuilder) -> i32 {
    info.push_colored_string("N/A".red());
    0
}
