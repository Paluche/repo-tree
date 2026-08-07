//! Configure the JuJutsu repository.

use std::error::Error;
use std::path::Path;

use super::command::JujutsuCommand;
use crate::config::Identity;

pub enum ConfigScope<'repo> {
    User,
    Repo(&'repo Path),
    Workspace(&'repo Path),
}

impl<'repo> ConfigScope<'repo> {
    fn new_config_command(&self) -> Result<JujutsuCommand, which::Error> {
        let mut cmd  = match self {
            Self::User => JujutsuCommand::global(),
            Self::Repo(repo_path) | Self::Workspace(repo_path) => {
                JujutsuCommand::repo(repo_path)
            }
        }?;

        cmd.arg("config");

        Ok(cmd)
    }

    fn get(&self, key: &str) -> Result<String, Box<dyn Error>> {
        self.new_config_command()?.arg("get").arg(key).output()
    }

    fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
        self.new_config_command()?
        .arg("set")
            .arg(match self {
                Self::User => "--user",
                Self::Repo(_) => "--repo",
                Self::Workspace(_) => "--Workspace",
            })
            .arg(key)
            .arg(value)
            .status()
    }
}

const USER_NAME_KEY: &str = "user.name";
const USER_EMAIL_KEY: &str = "user.email";

fn get_identity_internal(
    config: ConfigScope,
) -> Result<Option<Identity>, Box<dyn Error>> {
    Ok(
        if let Some(name) = config.get(USER_NAME_KEY)?
            && let Some(email) = config.get(USER_EMAIL_KEY)?
        {
            Some(Identity::new(name, email))
        } else {
            None
        },
    )
}

pub fn get_global_identity() -> Result<Option<Identity>, Box<dyn Error>> {
    get_identity_internal(ConfigScope::User)
}

pub fn get_identity(
    repo_path: &Path,
) -> Result<Option<Identity>, Box<dyn Error>> {
    get_identity_internal(ConfigScope::Repo(repo_path))
}

pub fn set_identity_internal(
    config: ConfigScope,
    identity: &Identity,
) -> Result<(), std::io::Error> {
    config.set(USER_NAME_KEY, &identity.name)?;
    config.set(USER_EMAIL_KEY, &identity.email)?;

    Ok(())
}

pub fn set_global_identity(identity: &Identity) -> Result<(), std::io::Error> {
    set_identity_internal(ConfigScope::User, identity)
}

pub fn set_identity(
    repo_path: &Path,
    identity: &Identity,
) -> Result<(), std::io::Error> {
    set_identity_internal(ConfigScope::Repo(repo_path), identity)
}

fn get_credentials() {}

fn set_credentials() {}

fn get_signing() {}

fn set_signing() {}
