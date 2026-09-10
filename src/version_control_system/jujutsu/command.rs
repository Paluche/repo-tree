//! Execute Jujutsu command.

use std::error::Error;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::process::Command;

use which::which;

use crate::error::CommandError;

/// Manage the execution of a Jujutsu command.
pub struct JujutsuCommand {
    /// Actual command.
    command: Command,
    /// Repository into which the command must be run
    repository: Option,
}

impl JujutsuCommand {
    /// Create a new Jujutsu command.
    pub fn global() -> Result<Self, which::Error> {
        Self::new_command().map(|command| Self {
            command,
            repository: None,
        })
    }

    pub fn repo<S: AsRef<OsStr>>(repository: S) -> Result<Self, which::Error> {
        Self::new_command().map(|mut command| {
            command.arg("--repository").arg(repository);
            Self {
                command,
                repository: Some(repository.as_ref().to_os_string()),
            }
        })
    }

    fn new_command() -> Result<Command, which::Error> {
        which("jj").map(|prog| Command::new(prog))
    }

    /// See [Command::arg()]
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.command.arg(arg);
        self
    }

    /// Ignore the
    pub fn ignore_working_copy(&mut self) -> &mut Self {
        self.arg("--ignore-working-copy")
    }

    pub fn repository<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.arg("--repository").arg(arg)
    }

    pub fn workspace_update_stale<S: AsRef<OsStr>>(
        arg: S,
    ) -> Result<(), Box<dyn Error>> {
        Self::new_command()?
            .arg("--repository")
            .arg(arg)
            .arg("workspace")
            .arg("update-stale")
            .status()?;
        Ok(())
    }

    pub fn _output_lines(
        &mut self,
        first_try: bool,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let output = self.command.output()?;
        let status = output.status;
        if !status.success() {
            if first_try
                && String::from_utf8_lossy(output.stderr.as_slice())
                    .contains("Run `jj workspace update-stale` to update it.")
            {
                if let Some(repository) = self.repository {
                    Self::workspace_update_stale(repository);
                }
                self._output_lines(false)
            } else {
                Err(Box::new(CommandError::new(
                    &self.command,
                    status,
                    Some(output),
                )))
            }
        } else {
            Ok(String::from_utf8(output.stdout)?
                .split("\n")
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect())
        }
    }

    pub fn output_lines(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        self._output_lines(true)
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
