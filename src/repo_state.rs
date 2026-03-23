use std::fmt::Display;

use colored::Colorize;

pub struct RepoState {
    has_unpushed_commits: bool,
    needs_restack: bool,
    has_conflicts: bool,
}

impl RepoState {
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

    pub fn is_ok(&self) -> bool {
        !(self.has_unpushed_commits || self.needs_restack || self.has_conflicts)
    }

    pub fn has_unpushed_commits(&self) -> bool {
        self.has_unpushed_commits
    }

    pub fn needs_restack(&self) -> bool {
        self.needs_restack
    }

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
