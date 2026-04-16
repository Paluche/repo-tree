//! Builder for prompt string.
use colored::Colorize;
use itertools::join;

use crate::config::Config;
use crate::config::IsEmpty;
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
        S: ToString + IsEmpty,
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
        write!(
            f,
            "{}{}{}{}{}{}",
            self.config.prompt.prefix,
            self.prompt.repository.vcs.short_display(self.config),
            self.config.prompt.separator,
            self.prompt.repository.id.remote.host(self.config).repr(),
            self.config.prompt.separator,
            self.prompt.repository.id.name.green()
        )?;

        for field in &self.prompt.fields {
            write!(f, "{}{field}", self.config.prompt.separator)?;
        }

        Ok(())
    }
}

#[allow(unused)]
/// Prompt field which contains a list.
pub struct PromptListField {
    /// List to build the field with.
    list: Vec<String>,
    /// Separator to separate the items from each other.
    separator: &'static str,
}

#[allow(unused)]
impl PromptListField {
    /// Create a new PromptListField.
    pub fn new(separator: &'static str) -> Self {
        Self {
            list: Vec::new(),
            separator,
        }
    }

    /// Extend the prompt line with a string.
    pub fn push<S>(&mut self, string: S)
    where
        S: ToString + IsEmpty,
    {
        if !string.is_empty() {
            self.list.push(string.to_string())
        }
    }
}

impl IsEmpty for PromptListField {
    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl std::fmt::Display for PromptListField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", join(self.list.iter(), self.separator))
    }
}
