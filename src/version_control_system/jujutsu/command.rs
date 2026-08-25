//! Execute Jujutsu command.

use std::error::Error;
use std::ffi::OsStr;
use std::process::Command;
use std::process::Output;

use which::which;

use crate::error::CommandError;

/// Manage the execution of a Jujutsu command.
pub struct JujutsuCommand {
    /// Actual command.
    command: Command,
}

impl JujutsuCommand {
    /// Create a new global Jujutsu command.
    pub fn global() -> Result<Self, which::Error> {
        Self::new_command().map(|mut command| {
            command.current_dir(std::env::home_dir().unwrap());
            Self { command }
        })
    }

    /// Create a new Jujutsu command to run in a specific repository. The
    /// repository will not be snapshot.
    pub fn repo<S: AsRef<OsStr>>(repository: S) -> Result<Self, which::Error> {
        Self::new_command().map(|mut command| {
            command
                .arg("--repository")
                .arg(&repository)
                .arg("--ignore-working-copy");
            Self { command }
        })
    }

    /// Create a new Jujutsu command to run in a specific repository. Where jj
    /// is allowed to snapshot the repository.
    pub fn repo_with_snapshot<S: AsRef<OsStr>>(
        repository: S,
    ) -> Result<Self, which::Error> {
        Self::new_command().map(|mut command| {
            command.arg("--repository").arg(&repository);
            Self { command }
        })
    }

    /// Instantiate a new jj command.
    fn new_command() -> Result<Command, which::Error> {
        which("jj").map(Command::new)
    }

    /// See [Command::arg()]
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.command.arg(arg);
        self
    }

    /// Get the output lines of the command.
    fn output(&mut self) -> Result<Output, Box<dyn Error>> {
        let output = self.command.output()?;
        let status = output.status;
        if !status.success() {
            Err(Box::new(CommandError::new(
                &self.command,
                status,
                Some(output),
            )))
        } else {
            Ok(output)
        }
    }

    /// Get the output lines of the command. Managing the case where the
    /// workspace is now stalled and needs to be updated.
    pub fn output_lines(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        let output = self.output()?;
        Ok(String::from_utf8(output.stdout)?
            .split("\n")
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect())
    }

    /// Execute the command and checks it succeeds. Managing the case where
    /// the workspace is now stalled and needs to be updated.
    pub fn status(&mut self) -> Result<(), Box<dyn Error>> {
        self.output().map(|_| ())
    }
}
