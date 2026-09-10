//! Refresh the repositories cache.
use clap::Args;

use crate::config::Config;
use crate::repo_tree::RepoTree;
use crate::ui::Ui;

/// Refresh the repositories cache.
#[derive(Args)]
pub struct RefreshCacheArgs {}

/// Execute the `rt refresh-cache` command.
pub fn run(config: &Config, ui: &Ui, _: RefreshCacheArgs) -> i32 {
    RepoTree::load(config, ui, true);
    0
}
