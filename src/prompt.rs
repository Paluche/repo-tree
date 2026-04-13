//! Builder for prompt string.
use std::fmt::Display;

use colored::ColoredString;
use colored::Colorize;

use crate::config::Config;
use crate::repository::Repository;

/// Context to build the prompt line.
pub struct Prompt {
    /// Current content of the prompt line.
    prompt: String,
    /// String to use to separate elements of the prompt line.
    sep: String,
}

impl Prompt {
    /// Instantiate a new Prompt for a repository.
    pub fn new(config: &Config, repository: &Repository) -> Self {
        let sep = format!("{}", "|".cyan());
        Self {
            prompt: format!(
                "{}{}{sep}{}{sep}{}",
                "┣━┫".cyan(),
                repository.vcs.short_display(),
                repository.id.remote.host(config).repr(),
                repository.id.name.green()
            ),
            sep,
        }
    }

    /// Extend the prompt line with a colored string, appending a separator
    /// before the content.
    pub fn push_colored_string(&mut self, colored_string: ColoredString) {
        if !colored_string.is_empty() {
            self.prompt
                .push_str(&format!("{}{}", self.sep, colored_string));
        }
    }

    /// Extend the prompt line with a string, appending a separator before the
    /// content.
    pub fn push_string(&mut self, string: &str) {
        if !string.is_empty() {
            self.prompt.push_str(&self.sep);
            self.prompt.push_str(string);
        }
    }
}

impl Display for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prompt)
    }
}
