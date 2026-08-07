//! Configure the JuJutsu repository.

use std::error::Error;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use jj_lib::repo::ReadonlyRepo;

use crate::config::Identity;

pub enum ConfigScope<'repo> {
    User,
    Repo(&'repo Path),
    Workspace(&'repo Path),
}

impl<'repo> ConfigScope<'repo> {
    fn new_config_command(&self) -> Command {
        let mut cmd = Command::new("jj");

        match self {
            Self::User => cmd.current_dir(std::env::home_dir().unwrap()),
            Self::Repo(repo_path) | Self::Workspace(repo_path) => {
                cmd.arg("--repository").arg(repo_path)
            }
        }
        .arg("--ignore-working-copy")
        .arg("config");

        cmd
    }

    fn get(&self, key: &str) -> Result<Option<String>, Box<dyn Error>> {
        let output = self.new_config_command().arg("get").arg(key).output()?;

        if let Some(code) = output.status.code() {
            if code == 1 {
                return Ok(None);
            } else if code == 0 {
                return Ok(Some(String::from_utf8(output.stdout)?));
            }
        }

        panic!(
            "Unexpected jj command failed with {:?}",
            output.status.code()
        );
    }

    fn set(&self, key: &str, value: &str) -> Result<(), std::io::Error> {
        let mut cmd = self.new_config_command();

        cmd.arg("set")
            .arg(match self {
                Self::User => "--user",
                Self::Repo(_) => "--repo",
                Self::Workspace(_) => "--Workspace",
            })
            .arg(key)
            .arg(value)
            .spawn()?;
        Ok(())
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
