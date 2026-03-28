//! Refresh the repositories cache.
use clap::Args;

use crate::config::Config;
use crate::repository::Repositories;

/// Refresh the repositories cache.
#[derive(Args)]
pub struct RefreshCacheArgs {}

/// Execute the `rt refresh-cache` command.
pub fn run(config: &Config, _: RefreshCacheArgs) -> i32 {
    Repositories::load(config, true);
    0
}
