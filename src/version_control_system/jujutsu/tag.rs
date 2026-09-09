//! Utils related to the tags.
//!
//! Note that the management of tags in jj in pretty incomplete. I would have
//! expected to have a state for local non-synced tags, but the tag list using
//! the template from CommitRef shows always the same thing. The tag is never
//! out of synced while it has not been pushed.
//! So we might have to do something later when things evolves and it will look
//! a lot like the bookmark mod, in the mean time we have only the tag name
//! which is worth information.
use colored::Colorize;

/// Get the reference short representation as tag.
pub fn get_tag_repr(tag_name: &str) -> String {
    tag_name.yellow().to_string()
}
