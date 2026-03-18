use std::fmt::Display;

use colored::Colorize;

pub struct RepoState {
    pub unpushed_commits: bool,
    pub needs_restack: bool,
    pub has_conflicts: bool,
}

impl RepoState {
    pub fn is_ok(&self) -> bool {
        !(self.unpushed_commits || self.needs_restack || self.has_conflicts)
    }
}

impl Display for RepoState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut empty = true;
        if self.unpushed_commits {
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
            write!(f, "{}", "has_conflicts".bright_red())?;
            empty = false;
        }

        if empty {
            write!(f, "{}", "OK".green())?;
        }

        Ok(())
    }
}
