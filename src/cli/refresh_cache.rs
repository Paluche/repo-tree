//! Refresh the repositories cache.
use clap::Args;

use crate::config::Config;
use crate::tree::RepoTree;

/// Refresh the repositories cache.
#[derive(Args)]
pub struct RefreshCacheArgs {}

/// Execute the `rt refresh-cache` command.
pub fn run(config: &Config, _: RefreshCacheArgs) -> i32 {
    RepoTree::load(config, true);
    0
}
