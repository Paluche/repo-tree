//! Define the repository identifier RepoId, it is obtained by parsing the URL
//! of the remote associated with the repository.
use std::fmt::Display;

use colored::Colorize;

/// State of a repository. Get information about the status of your job within
/// the repository.
pub struct RepoState {
    /// The repository has commits with changes which are not pushed to the
    /// remote.
    has_unpushed_commits: bool,
    /// Some branches needs to be rebased to be kept up-to-date.
    needs_restack: bool,
    /// Some commits have conflicts.
    has_conflicts: bool,
}

impl RepoState {
    /// Create a new RepoState structure.
    pub fn new(
        has_unpushed_commits: bool,
        needs_restack: bool,
        has_conflicts: bool,
    ) -> Self {
        Self {
            has_unpushed_commits,
            needs_restack,
            has_conflicts,
        }
    }

    /// Is the repository in an OK state?
    pub fn is_ok(&self) -> bool {
        !(self.has_unpushed_commits || self.needs_restack || self.has_conflicts)
    }

    /// Is there unpushed commits / changes in the repository which you should
    /// push?
    pub fn has_unpushed_commits(&self) -> bool {
        self.has_unpushed_commits
    }

    /// Does your branches needs to be rebased to be kept up-to-date?
    pub fn needs_restack(&self) -> bool {
        self.needs_restack
    }

    /// Is there conflicts to resolve?
    pub fn has_conflicts(&self) -> bool {
        self.has_conflicts
    }
}

impl Display for RepoState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut empty = true;
        if self.has_unpushed_commits {
            write!(f, "{}", "unpushed commits".purple())?;
            empty = false;
        }

        if self.needs_restack {
            if !empty {
                write!(f, ", ")?;
            }
            write!(f, "{}", "needs restack".yellow())?;
            empty = false;
        }

        if self.has_conflicts {
            if !empty {
                write!(f, ", ")?;
            }
            write!(f, "{}", "has conflicts".bright_red())?;
            empty = false;
        }

        if empty {
            write!(f, "{}", "OK".green())?;
        }

        Ok(())
    }
}
