//! Definition of errors struct used in the crate.
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Output;

use thiserror::Error;

use crate::tree_space::TreeSpace;
use crate::version_control_system::VersionControlSystem;

#[derive(Debug, Error)]
#[error("Command {0} failed with error {1}:\n{2}")]
/// A functionality is not implemented yet.
pub struct CommandError(pub String, pub i32, pub String);

impl CommandError {
    /// Create a new CommandError struct based on the command and output of the
    /// failed command.
    pub fn new(
        command: &Command,
        status: ExitStatus,
        output: Option<Output>,
    ) -> Self {
        let mut cmd = command.get_program().to_os_string();
        cmd.push(" ");
        for arg in command.get_args() {
            cmd.push("'");
            cmd.push(arg);
            cmd.push("' ");
        }
        CommandError(
            cmd.display().to_string(),
            status.code().unwrap_or(1),
            output
                .and_then(|o| String::from_utf8(o.stderr).ok())
                .unwrap_or("".to_string()),
        )
    }
}

#[derive(Debug, Error)]
#[error("{0} not implemented yet")]
/// A functionality is not implemented yet.
pub struct NotImplementedError(pub String);

#[derive(Debug, Error)]
#[error("Error parsing {0}")]
/// Error during the parsing of the remote URL.
pub struct ParseUrlError(pub String);

#[derive(Debug, Error)]
#[error("No repository found in {0}")]
/// No repository found.
pub struct NoRepositoryError(pub PathBuf);

#[derive(Debug, Error)]
#[error("Repository {0} should not be in the {1} tree-space")]
/// A repository cannot be placed in the given tree-space.
pub struct UnexpectedTreeSpaceError(pub String, pub String);

#[derive(Debug, Error)]
#[error("Missing host configuration for {0}")]
/// Error when trying to obtain configuration information about a host, which
/// has no configuration associated.
pub struct UnknownRemoteHostError(pub String);

#[derive(Debug, Error)]
#[error("No cache file to load")]
/// Error during the parsing of the remote URL.
pub struct NoCacheError();

#[derive(Debug, Error)]
#[error("Bad configuration: {0}")]
/// Error during the parsing of the configuration.
pub struct ConfigError(pub String);

#[derive(Debug, Error)]
#[error(
    "Unable to convert timestamp to obtain the last time {0} has been modified"
)]
/// Error during the retrieving of the last time a file has been modified.
pub struct GetLastModifiedError(pub String);

#[derive(Debug, Error)]
#[error("The API interaction with the forge {0} is not yet implemented.")]
/// Error during the retrieving of the last time a file has been modified.
pub struct UnimplementedForgeApi(pub String);

#[derive(Debug, Error)]
#[error("No {0} repository in: {1}")]
/// The directory is not a repository of the given version control system.
pub struct NotARepositoryError(pub VersionControlSystem, pub String);

#[derive(Debug, Error)]
#[error("TreeSpace::{0:?} cannot be converted into a WorkspaceTreeSpace")]
/// Error during the parsing of the configuration.
pub struct InvalidWorkspaceTreeSpace(pub TreeSpace);

#[derive(Debug, Error)]
#[error("TreeSpace::{0:?} cannot be converted into a ForceTreeSpace")]
/// Error during the parsing of the configuration.
pub struct InvalidForceTreeSpace(pub TreeSpace);

#[cfg(test)]
mod tests {
    use std::os::unix::process::ExitStatusExt;

    use super::*;

    /// Build the command `git status --short` without running it.
    fn command() -> Command {
        let mut command = Command::new("git");
        command.args(["status", "--short"]);
        command
    }

    #[test]
    fn command_error_quotes_each_argument() {
        let error =
            CommandError::new(&command(), ExitStatus::from_raw(2 << 8), None);

        assert_eq!(error.0, "git 'status' '--short' ");
        assert_eq!(error.1, 2);
        assert_eq!(error.2, "");
    }

    #[test]
    fn command_error_keeps_stderr() {
        let output = Output {
            status: ExitStatus::from_raw(128 << 8),
            stdout: b"ignored".to_vec(),
            stderr: b"fatal: not a git repository\n".to_vec(),
        };

        let error =
            CommandError::new(&command(), output.status, Some(output.clone()));

        assert_eq!(error.1, 128);
        assert_eq!(error.2, "fatal: not a git repository\n");
        assert_eq!(
            error.to_string(),
            "Command git 'status' '--short'  failed with error 128:\nfatal: \
             not a git repository\n"
        );
    }

    #[test]
    fn command_error_ignores_non_utf8_stderr() {
        let output = Output {
            status: ExitStatus::from_raw(1 << 8),
            stdout: vec![],
            stderr: vec![0xff, 0xfe],
        };

        let error = CommandError::new(&command(), output.status, Some(output));

        assert_eq!(error.2, "");
    }

    #[test]
    fn command_error_killed_by_signal_reports_code_1() {
        // Raw wait status of a process killed by SIGKILL.
        let error =
            CommandError::new(&command(), ExitStatus::from_raw(9), None);

        assert_eq!(error.1, 1);
    }

    #[test]
    fn unexpected_tree_space_error_names_the_tree_space() {
        let error =
            UnexpectedTreeSpaceError("owner/repo".into(), "local".into());

        assert_eq!(
            error.to_string(),
            "Repository owner/repo should not be in the local tree-space"
        );
    }

    #[test]
    fn not_a_repository_error_names_the_directory() {
        let error =
            NotARepositoryError(VersionControlSystem::Git, "/some/dir".into());

        assert_eq!(error.to_string(), "No git repository in: /some/dir");
    }
}
