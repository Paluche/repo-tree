//! Read and write the configuration of a Git repository.
use std::error::Error;

use crate::config::ResolvedIdentity;
use crate::version_control_system::identity::ConfigChange;
use crate::version_control_system::identity::ConfigValue;
use crate::version_control_system::identity::changes;
use crate::version_control_system::identity::git_settings;

/// Open the configuration of the repository itself, without the global and
/// system ones.
fn local_config(repo: &git2::Repository) -> Result<git2::Config, git2::Error> {
    repo.config()?.open_level(git2::ConfigLevel::Local)
}

/// Get a value from the repository configuration, with the type of the
/// expected value.
fn get(
    config: &git2::Config,
    key: &str,
    expected: &ConfigValue,
) -> Option<ConfigValue> {
    match expected {
        ConfigValue::String(_) => {
            config.get_string(key).ok().map(ConfigValue::String)
        }
        ConfigValue::Bool(_) => {
            config.get_bool(key).ok().map(ConfigValue::Bool)
        }
    }
}

/// Get the configuration changes the repository needs to use the identity.
pub fn identity_changes(
    repo: &git2::Repository,
    identity: &ResolvedIdentity,
) -> Result<Vec<ConfigChange>, Box<dyn Error>> {
    let config = local_config(repo)?;

    Ok(changes("git", git_settings(identity), |key, expected| {
        get(&config, key, expected)
    }))
}

/// Apply configuration changes in the repository configuration.
pub fn apply(
    repo: &git2::Repository,
    changes: &[ConfigChange],
) -> Result<(), Box<dyn Error>> {
    let mut config = local_config(repo)?;

    for change in changes {
        match &change.expected {
            ConfigValue::String(value) => config.set_str(change.key, value),
            ConfigValue::Bool(value) => config.set_bool(change.key, *value),
        }
        .map_err(|err| {
            format!(
                "Unable to set {} in {}: {err}",
                change.key,
                repo.path().display()
            )
        })?;
    }

    Ok(())
}
