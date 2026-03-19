//! Builder for prompt string.

use std::ops::Deref;

use colored::Colorize;

use crate::config::Config;
use crate::repository::Repository;

/// Context to build the prompt line.
pub struct Prompt<'repo> {
    /// Repository for which the prompt is for.
    repository: &'repo Repository,
    /// Fields of the prompt.
    fields: Vec<String>,
}

impl<'repo> Prompt<'repo> {
    /// Instantiate new Prompt for a repository.
    pub fn new(repository: &'repo Repository) -> Self {
        Self {
            repository,
            fields: Vec::new(),
        }
    }

    /// Extend the prompt line with a string.
    pub fn push<S>(&mut self, string: S)
    where
        S: ToString + Deref<Target = str>,
    {
        if !string.is_empty() {
            self.fields.push(string.to_string())
        }
    }

    /// Obtain a displayable struct representing the Prompt.
    pub fn display<'pb, 'config>(
        &'pb self,
        config: &'config Config,
    ) -> Display<'pb, 'repo, 'config> {
        Display {
            prompt: self,
            config,
        }
    }
}

/// Displayable struct representing the Prompt.
pub struct Display<'prompt, 'repo, 'config> {
    /// Prompt we are displaying.
    prompt: &'prompt Prompt<'repo>,
    /// Configuration customizing the prompt.
    config: &'config Config,
}

impl<'pb, 'repo, 'config> std::fmt::Display for Display<'pb, 'repo, 'config> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let separator = format!("{}", "|".cyan());
        write!(
            f,
            "{}{}{}{}{}{}",
            "┣━┫".cyan(),
            self.prompt.repository.vcs.short_display(),
            separator,
            self.prompt.repository.id.remote.host(self.config).repr(),
            separator,
            self.prompt.repository.id.name.green()
        )?;

        for field in &self.prompt.fields {
            write!(f, "{}{field}", separator)?;
        }

        Ok(())
    }
}
