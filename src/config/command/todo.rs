//! Configuration for specific todo command.

use serde::Deserialize;
use serde::Serialize;

/// Configuration for the `rt todo` command.
#[derive(Serialize, Deserialize, Default)]
pub struct TodoCommandConfig {
    /// List of ID of repositories to be ignored by the command.
    #[serde(default)]
    pub ignore: Vec<String>,
}
