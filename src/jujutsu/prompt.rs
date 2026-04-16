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
use crate::config::ColoredList;
use crate::config::Config;
use crate::config::IsEmpty;
use crate::config::JujutsuBookmarkConfig;
use crate::config::JujutsuPromptConfig;
use crate::prompt::Prompt;
use crate::prompt::PromptListField;

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
    fn get_repr<'config>(
        &self,
        config: &'config JujutsuBookmarkConfig,
    ) -> &'config ColoredList {
        match self {
            Self::Current => &config.current,
            Self::Parents => &config.parent,
            Self::Descendants => &config.descendants,
        }
    }
}

/// Build the list of bookmarks of the specified category for the prompt line.
fn list_bookmarks(
    config: &JujutsuBookmarkConfig,
    field: &mut PromptListField,
    category: BookmarkCategory,
    repo: &dyn Repo,
    current_commit: &CommitId,
) -> BackendResult<()> {
    field.push(
        category.get_repr(config).display(
            &category
                .get_bookmarks(repo, current_commit)?
                .map(|b| b.get_bookmark_repr())
                .collect::<Vec<String>>(),
        ),
    );
    Ok(())
}

/// Build the list of tags for the prompt line.
fn list_tags(
    config: &JujutsuPromptConfig,
    field: &mut PromptListField,
    repo: &dyn Repo,
    current_commit: &CommitId,
) -> BackendResult<()> {
    field.push(
        config.tags.display(
            &RevsetExpression::commits(vec![current_commit.to_owned()])
                .parents()
                .evaluate(repo)
                .map_err(|e| e.into_backend_error())?
                .iter()
                .flat_map(|r| {
                    let commit = r.unwrap();
                    repo.view()
                        .tags()
                        .filter_map(move |(name, lrrt)| {
                            Ref::try_new(name, &lrrt, &commit)
                        })
                        .map(|r| r.get_tag_repr())
                })
                .collect::<Vec<String>>(),
        ),
    );

    Ok(())
}

/// Internal method to build the prompt line for a Jujutsu repository.
fn prompt_internal(
    config: &Config,
    prompt: &mut Prompt,
    repo_path: &Path,
    repo: &dyn Repo,
    current_commit: &CommitId,
) -> Result<(), Box<dyn Error>> {
    let config = &config.prompt.jj;
    {
        let mut field = PromptListField::new(" ");

        list_bookmarks(
            &config.bookmark,
            &mut field,
            BookmarkCategory::Parents,
            repo,
            current_commit,
        )?;
        list_bookmarks(
            &config.bookmark,
            &mut field,
            BookmarkCategory::Current,
            repo,
            current_commit,
        )?;
        list_bookmarks(
            &config.bookmark,
            &mut field,
            BookmarkCategory::Descendants,
            repo,
            current_commit,
        )?;
        list_tags(config, &mut field, repo, current_commit)?;

        if field.is_empty() {
            prompt.push(&config.bookmark.none)
        } else {
            prompt.push(field)
        }
    }

    if has_conflicts(repo_path)? {
        prompt.push(&config.conflict);
    }

    Ok(())
}

/// Build the prompt line for a Jujutsu repository.
pub async fn prompt(
    config: &Config,
    prompt: &mut Prompt<'_>,
    repo_path: &Path,
) -> i32 {
    let repo = load(repo_path).await.unwrap();
    let Some(current_commit) =
        repo.view().get_wc_commit_id(WorkspaceName::DEFAULT)
    else {
        return 1;
    };

    if let Err(err) = prompt_internal(
        config,
        prompt,
        repo_path,
        repo.as_ref(),
        current_commit,
    ) {
        eprintln!("{err}");
        1
    } else {
        0
    }
}
