//! Execute Git command.

use std::error::Error;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::process::Command;

use which::which;

use crate::error::CommandError;
use crate::error::NotARepositoryError;
use crate::version_control_system::VersionControlSystem;

/// Manage the execution of a git command.
pub struct GitCommand {
    /// Actual command.
    command: Command,
    /// Repository for which is the command.
    repository: Option<OsString>,
}

impl GitCommand {
    /// Create a new global git command.
    pub fn global() -> Result<Self, which::Error> {
        Self::new_command().map(|mut command| {
            command.current_dir(std::env::home_dir().unwrap());
            Self {
                command,
                repository: None,
            }
        })
    }

    /// Create a new Git command to run in a specific repository.
    pub fn repo<S: AsRef<OsStr>>(repository: S) -> Result<Self, which::Error> {
        Self::new_command().map(|mut command| {
            command.arg("-C").arg(&repository);
            Self {
                command,
                repository: Some(repository.as_ref().to_os_string()),
            }
        })
    }

    /// Instantiate a new git command.
    fn new_command() -> Result<Command, which::Error> {
        which("git").map(Command::new)
    }

    /// See [Command::arg()]
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.command.arg(arg.as_ref());
        self
    }

    /// Get the output lines of the command.
    pub fn output_lines(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        let output = self.command.output()?;
        let status = output.status;
        if !status.success() {
            if status.code().unwrap_or(1) == 128
                && let Some(repository) = &self.repository
                && let Some(repository) = repository.to_str()
            {
                return Err(Box::new(NotARepositoryError(
                    VersionControlSystem::Git,
                    repository.to_string(),
                )));
            }

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

    /// Return an empty vector if the command fails.
    pub fn output_lines_fallible(
        &mut self,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        match self.output_lines() {
            Ok(v) => Ok(v),
            Err(err) => {
                if err.downcast_ref::<CommandError>().is_some() {
                    Ok(Vec::new())
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Execute the command and checks it succeeds. Managing the case where the
    /// workspace is now stalled and needs to be updated.
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
