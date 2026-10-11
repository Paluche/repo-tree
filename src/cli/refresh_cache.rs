//! Refresh the repositories cache.
use clap::Args;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::ui::Ui;

/// Refresh the repositories cache.
#[derive(Args)]
pub struct RefreshCacheArgs {}

/// Execute the `rt refresh-cache` command.
pub fn run(ui: &Ui<'_>, config: &Config, _: RefreshCacheArgs) -> i32 {
    RepoTree::load(ui, config, true);
    0
}
