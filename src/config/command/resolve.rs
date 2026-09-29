//! Configuration for resolve command.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

/// Configuration for the `rt resolve` command.
#[derive(Serialize, Deserialize, Default)]
pub struct ResolveCommandConfig {
    /// Resolution aliases.
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
}
