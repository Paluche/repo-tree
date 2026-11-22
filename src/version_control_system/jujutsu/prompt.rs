//! Build the prompt line for a Jujutsu repository.
use std::error::Error;
use std::path::Path;

use super::bookmark::Bookmark;
use super::bookmark::Bookmarks;
use super::bookmark::get_all_bookmarks;
use super::bookmark::get_bookmarks;
use super::repo_state::has_conflicts;
use super::repo_state::wc_has_conflicts;
use super::revset::RevSetOrder;
use super::tag::Tag;
use super::tag::Tags;
use super::tag::get_all_tags;
use super::tag::get_tags;
use crate::colors::ColoredList;
use crate::config::Config;
use crate::config::JujutsuBookmarkConfig;
use crate::config::JujutsuTagConfig;
use crate::prompt::Prompt;
use crate::prompt::PromptListField;

/// The different categories of references we are listing.
#[derive(Clone, Copy)]
enum ReferenceCategory {
    /// The reference is set at the current commit.
    Current,
    /// The reference is set to the direct parent of the current commit.
    Parent,
    /// The reference is set to a commit which is a descendant of the current
    /// commit.
    Descendants,
}

impl ReferenceCategory {
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

        get_bookmarks(repo_path, bookmarks, revset, order)
    }

    /// Get short representation logo to represent this category of bookmarks.
    fn get_bookmark_repr<'config>(
        &self,
        bookmark_config: &'config JujutsuBookmarkConfig,
    ) -> &'config ColoredList {
        match self {
            Self::Current => &bookmark_config.current,
            Self::Parent => &bookmark_config.parent,
            Self::Descendants => &bookmark_config.descendants,
        }
    }

    /// Get the tags associated with the current category.
    fn get_tags<'tags>(
        &self,
        repo_path: &Path,
        tags: &'tags Tags,
    ) -> Result<Vec<&'tags Tag>, Box<dyn Error>> {
        let (revset, order) = match self {
            Self::Current => ("@", RevSetOrder::default()),
            Self::Parent => ("@-", RevSetOrder::ParentFirst),
            Self::Descendants => ("@+::", RevSetOrder::ChildrenFirst),
        };

        get_tags(repo_path, tags, revset, order)
    }

    /// Get short representation logo to represent this category of tags.
    fn get_tag_repr<'config>(
        &self,
        tag_config: &'config JujutsuTagConfig,
    ) -> &'config ColoredList {
        match self {
            Self::Current => &tag_config.current,
            Self::Parent => &tag_config.parent,
            Self::Descendants => &tag_config.descendants,
        }
    }
}

/// Build the list of bookmarks of the specified category for the prompt line.
fn list_bookmarks(
    bookmark_config: &JujutsuBookmarkConfig,
    field: &mut PromptListField,
    category: ReferenceCategory,
    repo_path: &Path,
    bookmarks: &Bookmarks,
) -> Result<(), Box<dyn Error>> {
    field.push(
        category.get_bookmark_repr(bookmark_config).display(
            &category
                .get_bookmarks(repo_path, bookmarks)?
                .iter()
                .flat_map(|b| b.get_repr(bookmark_config))
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
) {
    prompt.push(
        config.deleted.display(
            &bookmarks
                .get_deleted()
                .iter()
                .map(|b| b.name.clone())
                .collect::<Vec<String>>(),
        ),
    );
}

/// Build the list of tags for the prompt line.
fn list_tags(
    tag_config: &JujutsuTagConfig,
    field: &mut PromptListField,
    category: ReferenceCategory,
    repo_path: &Path,
    tags: &Tags,
) -> Result<(), Box<dyn Error>> {
    field.push(
        category.get_tag_repr(tag_config).display(
            &category
                .get_tags(repo_path, tags)?
                .iter()
                .flat_map(|t| t.get_repr(tag_config))
                .collect::<Vec<String>>(),
        ),
    );
    Ok(())
}

/// Build the list of deleted Tags pending some action.
fn list_deleted_tags(
    config: &JujutsuTagConfig,
    prompt: &mut Prompt<'_>,
    tags: &Tags,
) {
    prompt.push(
        config.deleted.display(
            &tags
                .get_deleted()
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<String>>(),
        ),
    );
}

/// Build the prompt line for a Jujutsu repository.
pub fn prompt(
    config: &Config,
    prompt: &mut Prompt<'_>,
    repo_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let prompt_config = &config.prompt.jj;
    {
        let mut field = PromptListField::new(" ");
        let bookmarks = get_all_bookmarks(repo_path)?;

        list_bookmarks(
            &prompt_config.bookmark,
            &mut field,
            ReferenceCategory::Parent,
            repo_path,
            &bookmarks,
        )?;
        list_bookmarks(
            &prompt_config.bookmark,
            &mut field,
            ReferenceCategory::Current,
            repo_path,
            &bookmarks,
        )?;
        list_bookmarks(
            &prompt_config.bookmark,
            &mut field,
            ReferenceCategory::Descendants,
            repo_path,
            &bookmarks,
        )?;

        let tags = get_all_tags(repo_path)?;
        list_tags(
            &prompt_config.tag,
            &mut field,
            ReferenceCategory::Parent,
            repo_path,
            &tags,
        )?;
        list_tags(
            &prompt_config.tag,
            &mut field,
            ReferenceCategory::Current,
            repo_path,
            &tags,
        )?;
        list_tags(
            &prompt_config.tag,
            &mut field,
            ReferenceCategory::Descendants,
            repo_path,
            &tags,
        )?;

        prompt.push(field);

        list_deleted_bookmarks(&prompt_config.bookmark, prompt, &bookmarks);
        list_deleted_tags(&prompt_config.tag, prompt, &tags);
    }

    if wc_has_conflicts(repo_path)? {
        prompt.push(&prompt_config.wc_conflict);
    } else if has_conflicts(repo_path)? {
        prompt.push(&prompt_config.conflict);
    }

    Ok(())
}
