//! Read and write the configuration of a Jujutsu repository.
use std::error::Error;
use std::path::Path;

use super::command::JujutsuCommand;
use crate::config::ResolvedIdentity;
use crate::version_control_system::identity::ConfigChange;
use crate::version_control_system::identity::ConfigValue;
use crate::version_control_system::identity::changes;
use crate::version_control_system::identity::jujutsu_settings;

/// Read the configuration set at the repository level.
fn repo_config(repo_path: &Path) -> Result<toml::Table, Box<dyn Error>> {
    // The listing is made of `dotted.key = value` lines, which is TOML.
    let output = JujutsuCommand::repo(repo_path)?
        .arg("config")
        .arg("list")
        .arg("--repo")
        .output()?;

    Ok(output.parse::<toml::Table>().map_err(|err| {
        format!("Unable to parse the Jujutsu repository configuration: {err}")
    })?)
}

/// Get a value from a configuration table, from its dotted key.
fn get(table: &toml::Table, key: &str) -> Option<ConfigValue> {
    let mut parts = key.split('.');
    let mut value = table.get(parts.next()?)?;

    for part in parts {
        value = value.as_table()?.get(part)?;
    }

    Some(match value {
        toml::Value::String(value) => ConfigValue::String(value.clone()),
        toml::Value::Boolean(value) => ConfigValue::Bool(*value),
        other => ConfigValue::String(other.to_string()),
    })
}

/// Get the configuration changes the repository needs to use the identity.
pub fn identity_changes(
    repo_path: &Path,
    identity: &ResolvedIdentity,
) -> Result<Vec<ConfigChange>, Box<dyn Error>> {
    let config = repo_config(repo_path)?;

    Ok(changes("jj", jujutsu_settings(identity), |key, _| {
        get(&config, key)
    }))
}

/// Apply configuration changes at the repository level.
pub fn apply(
    repo_path: &Path,
    changes: &[ConfigChange],
) -> Result<(), Box<dyn Error>> {
    for change in changes {
        // The value is parsed as TOML, so pass it as TOML: a name like
        // `true` must stay a string.
        let value = match &change.expected {
            ConfigValue::String(value) => {
                toml::Value::String(value.clone()).to_string()
            }
            ConfigValue::Bool(value) => value.to_string(),
        };

        JujutsuCommand::repo(repo_path)?
            .arg("config")
            .arg("set")
            .arg("--repo")
            .arg(change.key)
            .arg(value)
            .status()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_dotted_keys() {
        let table: toml::Table = concat!(
            "user.name = \"Name\"\n",
            "revsets.git-sign-on-push = \"none()\"\n",
            "ui.paginate = false\n",
        )
        .parse()
        .expect("valid TOML");

        assert_eq!(
            get(&table, "user.name"),
            Some(ConfigValue::String("Name".to_string()))
        );
        assert_eq!(
            get(&table, "revsets.git-sign-on-push"),
            Some(ConfigValue::String("none()".to_string()))
        );
        assert_eq!(get(&table, "ui.paginate"), Some(ConfigValue::Bool(false)));
        assert_eq!(get(&table, "user.email"), None);
        assert_eq!(get(&table, "user.name.first"), None);
    }
}
