//! Storage of credentials.
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::config::config_dir;

/// Path to the credentials cache file.
fn credentials_file() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap())
        .join(".config")
        .join("repo-tree")
        .join("credentials.toml")
}

/// Credential pair to authenticate to a Git remote through https.
#[derive(Serialize, Deserialize)]
struct Credential {
    /// Username to use to authenticate.
    username: String,

    /// Associated password.
    /// TODO: Support storage of the credential in a vault.
    password: String,
}

/// Map of credential.
#[derive(Serialize, Deserialize, Default)]
struct Credentials {
    credentials: BTreeMap<String, Credential>,
}

impl Credentials {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = config_dir()?.join("credentials.toml");
        Ok(if config_path.is_file() {
            toml::from_str(&fs::read_to_string(&config_path)?)?
        } else {
            Credentials::default()
        })
    }

    pub fn get(&self, id: &str) -> Option<&Credential> {
        self.credentials.get(id)
    }
}
