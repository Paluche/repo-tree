//! Execute Jujutsu command.

use std::error::Error;
use std::ffi::OsStr;
use std::process::Command;

use which::which;

use crate::error::CommandError;

/// Manage the execution of a Jujutsu command.
pub struct JujutsuCommand {
    /// Actual command.
    command: Command,
}

impl JujutsuCommand {
    /// Create a new Jujutsu command.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            command: Command::new(which("jj")?),
        })
    }

    /// See [Command::arg()]
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.command.arg(arg.as_ref());
        self
    }

    /// Ignore the
    pub fn ignore_working_copy(&mut self) -> &mut Self {
        self.arg("--ignore-working-copy")
    }

    pub fn repository<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.arg("--repository").arg(arg)
    }

    pub fn output_lines(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        let output = self.command.output()?;
        let status = output.status;
        if !status.success() {
            return Err(Box::new(CommandError::new(
                &self.command,
                status,
                Some(output),
            )));
        }

        Ok(String::from_utf8(output.stdout)?
            .split("\n")
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect())
    }

    pub fn status(&mut self) -> Result<(), Box<dyn Error>> {
        let status = self.command.status()?;

        if !status.success() {
            return Err(Box::new(CommandError::new(
                &self.command,
                status,
                None,
            )));
        }

        Ok(())
    }
}
