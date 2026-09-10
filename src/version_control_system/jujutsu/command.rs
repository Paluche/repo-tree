//! Execute Jujutsu command.

use std::error::Error;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::process::Command;
use std::process::Output;

use which::which;

use crate::error::CommandError;
use crate::ui::Ui;

/// Manage the execution of a Jujutsu command.
pub struct JujutsuCommand {
    /// Actual command.
    command: Command,
    /// Repository into which the command must be run
    repository: Option<OsString>,
}

impl JujutsuCommand {
    /// Create a new global Jujutsu command.
    pub fn global() -> Result<Self, which::Error> {
        Self::new_command().map(|mut command| {
            command.current_dir(std::env::home_dir().unwrap());
            Self {
                command,
                repository: None,
            }
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
            Self {
                command,
                repository: Some(repository.as_ref().to_os_string()),
            }
        })
    }

    /// Create a new Jujutsu command to run in a specific repository. Where jj
    /// is allowed to snapshot the repository.
    #[expect(dead_code)]
    pub fn repo_with_snapshot<S: AsRef<OsStr>>(
        repository: S,
    ) -> Result<Self, which::Error> {
        Self::new_command().map(|mut command| {
            command.arg("--repository").arg(&repository);
            Self {
                command,
                repository: Some(repository.as_ref().to_os_string()),
            }
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

    /// Update a potential stale workspace.
    pub fn workspace_update_stale<S: AsRef<OsStr>>(
        repository: S,
    ) -> Result<(), Box<dyn Error>> {
        Self::new_command()?
            .arg("--repository")
            .arg(repository)
            .arg("workspace")
            .arg("update-stale")
            .status()?;
        Ok(())
    }

    /// Get the output lines of the command.
    fn output(
        &mut self,
        ui: &Ui<'_>,
        first_try: bool,
    ) -> Result<Output, Box<dyn Error>> {
        let output = self.command.output()?;
        let status = output.status;
        if !status.success() {
            if first_try
                && String::from_utf8_lossy(output.stderr.as_slice())
                    .contains("Run `jj workspace update-stale` to update it.")
            {
                if let Some(repository) = &self.repository {
                    ui.hint("Updating stalled workspace...");
                    Self::workspace_update_stale(repository)?;
                }
                self.output(ui, false)
            } else {
                Err(Box::new(CommandError::new(
                    &self.command,
                    status,
                    Some(output),
                )))
            }
        } else {
            Ok(output)
        }
    }

    /// Get the output lines of the command. Managing the case where the
    /// workspace is now stalled and needs to be updated.
    pub fn output_lines(
        &mut self,
        ui: &Ui<'_>,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let output = self.output(ui, true)?;
        Ok(String::from_utf8(output.stdout)?
            .split("\n")
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect())
    }

    /// Execute the command and checks it succeeds. Managing the case where
    /// the workspace is now stalled and needs to be updated.
    pub fn status(&mut self, ui: &Ui<'_>) -> Result<(), Box<dyn Error>> {
        self.output(ui, true).map(|_| ())
    }
}
