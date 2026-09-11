//! Build the prompt line for a Jujutsu repository.
use std::error::Error;
use std::path::Path;

use super::bookmark::Bookmark;
use super::bookmark::Bookmarks;
use super::bookmark::get_bookmarks;
use super::repo_state::has_conflicts;
use super::repo_state::wc_has_conflicts;
use super::revset;
use super::revset::RevSetOrder;
use super::tag::get_tag_repr;
use crate::colors::ColoredList;
use crate::colors::IsEmpty;
use crate::config::Config;
use crate::config::JujutsuBookmarkConfig;
use crate::config::JujutsuPromptConfig;
use crate::prompt::Prompt;
use crate::prompt::PromptListField;

/// The different categories of bookmarks we are listing.
enum BookmarkCategory {
    /// The bookmark is set at the current commit.
    Current,
    /// The bookmark is set to the direct parent of the current commit.
    Parent,
    /// The bookmark is set to a commit which is a descendant of the current
    /// commit.
    Descendants,
}

impl BookmarkCategory {
    /// Get the bookmarks associated with the current category.
    fn get_bookmarks<'bookmarks>(
        &self,
        repo_path: &Path,
        bookmarks: &'bookmarks Bookmarks,
    ) -> Result<Vec<&'bookmarks Bookmark>, Box<dyn Error>> {
        let (revset, order) = match self {
            Self::Current => ("@", RevSetOrder::default()),
            Self::Parent => ("@-", RevSetOrder::ParentFirst),
            Self::Descendants => ("@+::", RevSetOrder::ChildrenFirst),
        };

        Ok(revset::list_bookmarks(repo_path, revset, order)?
            .iter()
            .map(|name| bookmarks.get(name).unwrap())
            .collect())
    }

    /// Get short representation logo to represent this category of bookmarks.
    fn get_repr<'config>(
        &self,
        config: &'config JujutsuBookmarkConfig,
    ) -> &'config ColoredList {
        match self {
            Self::Current => &config.current,
            Self::Parent => &config.parent,
            Self::Descendants => &config.descendants,
        }
    }
}

/// Build the list of bookmarks of the specified category for the prompt line.
fn list_bookmarks(
    config: &JujutsuBookmarkConfig,
    field: &mut PromptListField,
    category: BookmarkCategory,
    repo_path: &Path,
    bookmarks: &Bookmarks,
) -> Result<(), Box<dyn Error>> {
    field.push(
        category.get_repr(config).display(
            &category
                .get_bookmarks(repo_path, bookmarks)?
                .iter()
                .flat_map(|b| b.get_repr())
                .collect::<Vec<String>>(),
        ),
    );
    Ok(())
}

/// Build the list of deleted Bookmarks pending some action.
fn list_deleted_bookmarks(
    config: &JujutsuBookmarkConfig,
    prompt: &mut Prompt<'_>,
    bookmarks: &Bookmarks,
) -> Result<(), Box<dyn Error>> {
    prompt.push(
        config.deleted.display(
            &bookmarks
                .iter()
                .filter_map(|(name, bookmark)| {
                    bookmark.deleted.then_some(name.clone())
                })
                .collect::<Vec<String>>(),
        ),
    );
    Ok(())
}

/// Build the list of tags for the prompt line.
fn list_tags(
    config: &JujutsuPromptConfig,
    field: &mut PromptListField,
    repo_path: &Path,
) -> Result<(), Box<dyn Error>> {
    field.push(
        config.tags.display(
            &revset::list_tags(repo_path, "@-", RevSetOrder::default())?
                .iter()
                .map(|name| get_tag_repr(name))
                .collect::<Vec<String>>(),
        ),
    );

    Ok(())
}

/// Internal method to build the prompt line for a Jujutsu repository.
fn prompt_internal(
    config: &Config,
    prompt: &mut Prompt<'_>,
    repo_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let config = &config.prompt.jj;
    {
        let mut field = PromptListField::new(" ");
        let bookmarks = get_bookmarks(repo_path)?;

        list_bookmarks(
            &config.bookmark,
            &mut field,
            BookmarkCategory::Parent,
            repo_path,
            &bookmarks,
        )?;
        list_bookmarks(
            &config.bookmark,
            &mut field,
            BookmarkCategory::Current,
            repo_path,
            &bookmarks,
        )?;
        list_bookmarks(
            &config.bookmark,
            &mut field,
            BookmarkCategory::Descendants,
            repo_path,
            &bookmarks,
        )?;
        list_tags(config, &mut field, repo_path)?;

        if field.is_empty() {
            prompt.push(&config.bookmark.none)
        } else {
            prompt.push(field)
        }

        list_deleted_bookmarks(&config.bookmark, prompt, &bookmarks)?;
    }

    if wc_has_conflicts(repo_path)? {
        prompt.push(&config.wc_conflict);
    } else if has_conflicts(repo_path)? {
        prompt.push(&config.conflict);
    }

    Ok(())
}

/// Build the prompt line for a Jujutsu repository.
pub fn prompt(
    config: &Config,
    prompt: &mut Prompt<'_>,
    repo_path: &Path,
) -> i32 {
    if let Err(err) = prompt_internal(config, prompt, repo_path) {
        eprintln!("{err}");
        1
    } else {
        0
    }
}
