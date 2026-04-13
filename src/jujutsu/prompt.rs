//! Build the prompt line for a Jujutsu repository.
use std::error::Error;
use std::path::Path;

use colored::Colorize;
use jj_lib::backend::BackendResult;
use jj_lib::backend::CommitId;
use jj_lib::op_store::LocalRemoteRefTarget;
use jj_lib::ref_name::RefName;
use jj_lib::ref_name::WorkspaceName;
use jj_lib::repo::Repo;
use jj_lib::revset::RevsetExpression;

use super::load;
use super::repo_state::has_conflicts;
use crate::prompt::Prompt;

/// Status of a reference.
struct Ref {
    /// Name of the reference.
    name: String,
    /// If the reference has been modified compared to its known remote state.
    modified: bool,
    /// If the reference exists only locally.
    local_only: bool,
    /// If the reference exists only remotely, you will find in the Option the
    /// name of the remote.
    remote_only: Option<String>,
}

impl Ref {
    /// Try to instantiate a new Ref.
    fn try_new(
        name: &RefName,
        lrrt: &LocalRemoteRefTarget,
        current_commit: &CommitId,
    ) -> Option<Self> {
        let local_target = lrrt.local_target.as_normal();
        let local_only = lrrt.remote_refs.is_empty();
        let remote_only = if let Some(local_target) = local_target {
            (local_target == current_commit).then_some(None)?
        } else {
            Some(lrrt.remote_refs.iter().find_map(
                |&(remote_name, remote_ref)| {
                    remote_ref.target.as_normal().map(|c| {
                        (c == current_commit)
                            .then_some(remote_name.as_str().to_string())
                    })
                },
            )??)
        };

        let modified = local_target.is_some_and(|l| {
            lrrt.remote_refs.iter().any(|&(_, remote_ref)| {
                remote_ref.target.as_normal().is_some_and(|c| c != l)
            })
        });

        Some(Self {
            name: name.as_str().to_string(),
            modified,
            local_only,
            remote_only,
        })
    }

    /// Get the reference short representation as bookmark.
    fn get_bookmark_repr(&self) -> String {
        if let Some(remote) = &self.remote_only {
            format!("{}@{}", self.name.as_str(), remote).purple()
        } else if self.local_only {
            self.name.as_str().bright_green()
        } else {
            format!(
                "{}{}",
                self.name.as_str(),
                if self.modified { "*" } else { "" }
            )
            .bright_purple()
        }
        .to_string()
    }

    /// Get the reference short representation as tag.
    fn get_tag_repr(&self) -> String {
        if let Some(remote) = &self.remote_only {
            format!("{}@{}", self.name.as_str(), remote)
        } else if self.local_only {
            self.name.as_str().to_string()
        } else {
            format!(
                "{}{}",
                self.name.as_str(),
                if self.modified { "*" } else { "" }
            )
        }
        .yellow()
        .to_string()
    }
}

/// The different categories of bookmarks we are listing.
enum BookmarkCategory {
    /// The bookmark is set at the current commit.
    Current,
    /// The bookmark is set to the direct parent of the current commit.
    Parents,
    /// The bookmark is set to a commit which is a descendant of the current
    /// commit.
    Descendants,
}

impl BookmarkCategory {
    /// Get the bookmarks associated with the current category.
    fn get_bookmarks(
        &self,
        repo: &dyn Repo,
        current_commit: &CommitId,
    ) -> BackendResult<impl Iterator<Item = Ref>> {
        let revset = RevsetExpression::commits(vec![current_commit.to_owned()]);

        Ok(match self {
            Self::Current => revset,
            Self::Parents => revset.parents(),
            Self::Descendants => revset.descendants_at(1).descendants(),
        }
        .evaluate(repo)
        .map_err(|e| e.into_backend_error())?
        .iter()
        .flat_map(|r| {
            let commit = r.unwrap();
            repo.view().bookmarks().filter_map(move |(name, lrrt)| {
                Ref::try_new(name, &lrrt, &commit)
            })
        }))
    }

    /// Get short representation logo to represent this category of bookmarks.
    fn get_repr(&self) -> String {
        match self {
            Self::Current => "󰫍".bright_blue(),
            Self::Parents => "󰫍".yellow(),
            Self::Descendants => "󰫎".bright_blue(),
        }
        .to_string()
    }
}

/// Build the list of bookmarks of the specified category for the prompt line.
fn list_bookmarks(
    category: BookmarkCategory,
    repo: &dyn Repo,
    current_commit: &CommitId,
    buffer: &mut String,
) -> BackendResult<()> {
    let mut bookmarks = category.get_bookmarks(repo, current_commit)?;

    if let Some(bookmark) = bookmarks.next() {
        if !buffer.is_empty() {
            buffer.push(' ');
        }

        buffer.push_str(&category.get_repr());
        buffer.push_str(&bookmark.get_bookmark_repr());

        for bookmark in bookmarks {
            buffer.push_str(&"🞍".bright_blue());
            buffer.push_str(&bookmark.get_bookmark_repr());
        }
    }

    Ok(())
}

/// Build the list of tags for the prompt line.
fn list_tags(
    repo: &dyn Repo,
    current_commit: &CommitId,
    buffer: &mut String,
) -> BackendResult<()> {
    let mut tags = RevsetExpression::commits(vec![current_commit.to_owned()])
        .parents()
        .evaluate(repo)
        .map_err(|e| e.into_backend_error())?
        .iter()
        .flat_map(|r| {
            let commit = r.unwrap();
            repo.view().tags().filter_map(move |(name, lrrt)| {
                Ref::try_new(name, &lrrt, &commit)
            })
        });

    if let Some(tag) = tags.next() {
        if !buffer.is_empty() {
            buffer.push(' ');
        }

        buffer.push_str(&"".yellow());
        buffer.push_str(&tag.get_tag_repr());

        for tag in tags {
            buffer.push_str(&"🞍".bright_blue());
            buffer.push_str(&tag.get_tag_repr());
        }
    }

    Ok(())
}

/// Internal method to build the prompt line for a Jujutsu repository.
fn prompt_internal(
    repo_path: &Path,
    repo: &dyn Repo,
    current_commit: &CommitId,
    info: &mut Prompt,
) -> Result<(), Box<dyn Error>> {
    let mut buffer = String::new();

    list_bookmarks(
        BookmarkCategory::Parents,
        repo,
        current_commit,
        &mut buffer,
    )?;
    list_bookmarks(
        BookmarkCategory::Current,
        repo,
        current_commit,
        &mut buffer,
    )?;
    list_bookmarks(
        BookmarkCategory::Descendants,
        repo,
        current_commit,
        &mut buffer,
    )?;
    list_tags(repo, current_commit, &mut buffer)?;

    info.push_string(&if buffer.is_empty() {
        "󰫌".bright_black().to_string()
    } else {
        buffer
    });

    if has_conflicts(repo_path)? {
        info.push_colored_string("󰝧".bright_red())
    }

    Ok(())
}

/// Build the prompt line for a Jujutsu repository.
pub async fn prompt(repo_path: &Path, info: &mut Prompt) -> i32 {
    let repo = load(repo_path).await.unwrap();
    let Some(current_commit) =
        repo.view().get_wc_commit_id(WorkspaceName::DEFAULT)
    else {
        return 1;
    };

    if let Err(err) =
        prompt_internal(repo_path, repo.as_ref(), current_commit, info)
    {
        eprintln!("{err}");
        1
    } else {
        0
    }
}
