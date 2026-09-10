//! Operations related to the user interface.
use std::fmt::Display;

use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;

use crate::config::Config;

/// Interaction with the user through printing meaning full messages.
pub struct Ui<'config> {
    /// Configuration.
    config: &'config Config,
    /// List of messages that has already been printed to not overflow the
    /// output.
    printed_messages: Vec<String>,
}

impl<'config> Ui<'config> {
    /// Create a new instance of the struct.
    pub fn new(config: &'config Config) -> Self {
        Self {
            config,
            printed_messages: Vec::new(),
        }
    }

    /// Clear the current stderr line.
    pub fn clear_current_line(&self) {
        eprint!("\r{}", Clear(ClearType::CurrentLine));
    }

    /// Clear the current stderr line and print something in its place.
    pub fn clear_and_print_current_line<S>(&self, msg: S)
    where
        S: Display,
    {
        eprint!("\r{}{}", Clear(ClearType::CurrentLine), msg);
    }

    /// Print a information message (no prefix).
    pub fn info<S>(&self, msg: S)
    where
        S: Display,
    {
        eprintln!("{msg}")
    }

    /// Print a hint message.
    pub fn hint<S>(&self, msg: S)
    where
        S: Display,
    {
        eprintln!("{}: {msg}", self.config.ui.hint);
    }

    /// Print a warning message.
    pub fn warning<S>(&self, msg: S)
    where
        S: Display,
    {
        eprintln!("{}: {msg}", self.config.ui.warning);
    }

    /// Print an error message.
    pub fn error<S>(&self, msg: S)
    where
        S: Display,
    {
        eprintln!("{}: {msg}", self.config.ui.error);
    }

    /// Print a hint message. Only once per execution.
    pub fn hint_once<S>(&mut self, msg: S)
    where
        S: Display,
    {
        let msg = msg.to_string();
        if !self.printed_messages.contains(&msg) {
            self.hint(&msg);
            self.printed_messages.push(msg);
        }
    }
}
