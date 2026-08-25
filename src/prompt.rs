//! Builder for prompt string.

use std::fmt::Display;

use itertools::join;

use crate::config::Config;
use crate::repository::Repository;
use crate::repository::Workspace;

/// Context to build the prompt line.
pub struct Prompt<'repo> {
    /// Repository for which the prompt is for.
    repository: &'repo Repository,
    /// Exact workspace of the repository for which the prompt is for.
    workspace: &'repo Workspace,
    /// Fields of the prompt.
    fields: Vec<String>,
}

impl<'repo> Prompt<'repo> {
    /// Instantiate new Prompt for a repository.
    pub fn new(
        repository: &'repo Repository,
        workspace: &'repo Workspace,
    ) -> Self {
        Self {
            repository,
            workspace,
            fields: Vec::new(),
        }
    }

    /// Extend the prompt line with a string.
    pub fn push<S>(&mut self, string: S)
    where
        S: ToString,
    {
        let string = string.to_string();
        if !string.is_empty() {
            self.fields.push(string.to_string())
        }
    }

    /// Obtain a displayable struct representing the Prompt.
    pub fn display<'prompt, 'config>(
        &'prompt self,
        config: &'config Config,
    ) -> PromptDisplay<'prompt, 'repo, 'config> {
        PromptDisplay {
            prompt: self,
            config,
        }
    }
}

/// Displayable struct representing the Prompt.
pub struct PromptDisplay<'prompt, 'repo, 'config> {
    /// Prompt we are displaying.
    prompt: &'prompt Prompt<'repo>,
    /// Configuration customizing the prompt.
    config: &'config Config,
}

impl<'prompt, 'repo, 'config> Display
    for PromptDisplay<'prompt, 'repo, 'config>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            self.config.prompt.prefix,
            self.prompt.repository.vcs.short_display(self.config),
        )?;

        if let Some(tree_space) = &self.prompt.workspace.tree_space() {
            write!(
                f,
                "{}{}",
                self.config.prompt.separator,
                tree_space.repr(self.config)
            )?;
        }

        if let Some(repr) =
            self.prompt.repository.id.remote_host_repr(self.config)
        {
            write!(f, "{}{}", self.config.prompt.separator, repr)?;
        }

        write!(
            f,
            "{}{}",
            self.config.prompt.separator,
            self.config
                .prompt
                .id
                .colorize(&self.prompt.repository.id.name)
        )?;

        for field in &self.prompt.fields {
            write!(f, "{}{field}", self.config.prompt.separator)?;
        }

        Ok(())
    }
}

/// Prompt field which contains a list.
pub struct PromptListField {
    /// List to build the field with.
    list: Vec<String>,
    /// Separator to separate the items from each other.
    separator: &'static str,
}

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
        S: ToString,
    {
        let string = string.to_string();
        if !string.is_empty() {
            self.list.push(string);
        }
    }

    /// Is the list empty?
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl Display for PromptListField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", join(self.list.iter(), self.separator))
    }
}
