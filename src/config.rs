//! Format of the configuration file.
//! Should be located in `${XDG_CONFIG_HOME}/repo-tree/config.toml`.
//! If `XDG_CONFIG_HOME` is not set, then we will use the value
//! `${HOME}/.config` in place.
//!
//! The TOML configuration file has the following syntax:
//! ```toml
//! [hosts."URL"]
//! name = 'host_pretty_name'
//! dir_name: 'host_dir_name_in_tree'  # Optional, defaults to 'name' value.
//! repr = [PROMPT REPRESENTATION]  # Optional, defaults to 'name' value.
//! repr_color = 'prompt representation color'  # Optional, as int (ANSI color) or string
//!                                             # (literal), defaults to no color
//! [local]  # Optional
//! name = 'host_pretty_name'  # Defaults, to 'local'.
//! dir_name: 'host_dir_name_in_tree'  # Optional, defaults to 'name' value.
//! repr = [PROMPT REPRESENTATION]  # Optional, defaults to 'name' value.
//! repr_color = 'prompt representation color'  # Optional, as int (ANSI color) or string
//!                                             # (literal), defaults to no color
//! [command.resolve.aliases]
//! alias_name = 'full/repository/id'
//!
//! [command.todo]
//! ignore = [  # List of repositories to ignore.
//!   'full/repository/id'
//! ]
//!
//! [command.clone]
//! vcs = '' # Default VCS to clone: 'jujutsu', 'git' or 'jujutsu-git' (default)
//!   dir_name: <LOCAL REPOS DIR NAME IN TREE>
//! ```

use core::str::FromStr;
use std::{
    collections::HashMap,
    env,
    error::Error,
    ffi::OsStr,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use clap::builder::StyledStr;
use clap_complete::engine::CompletionCandidate;
use colored::{self, Colorize};
use serde::Deserialize;

use crate::VersionControlSystem;

#[derive(Default, Clone, Debug, PartialEq)]
struct Color {
    color: Option<colored::Color>,
}

impl FromStr for Color {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            color: Some(colored::Color::from_str(s)?),
        })
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        Self {
            color: Some(colored::Color::AnsiColor(value)),
        }
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ColorVisitor;

        impl<'de> serde::de::Visitor<'de> for ColorVisitor {
            type Value = Color;

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Color::from_str(value).map_err(|_| {
                    E::custom(format!("Invalid color string: {value}"))
                })
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                u8::try_from(value)
                    .map_err(|_| {
                        E::custom(format!(
                            "ANSI Color value out of range: {value}"
                        ))
                    })
                    .map(Color::from)
            }

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter
                    .write_str("a string or an integer representing a color")
            }
        }

        deserializer.deserialize_any(ColorVisitor)
    }
}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(match self.color {
            Some(c) => match c {
                colored::Color::Black => 0,
                colored::Color::Red => 1,
                colored::Color::Green => 2,
                colored::Color::Yellow => 3,
                colored::Color::Blue => 4,
                colored::Color::Magenta => 5,
                colored::Color::Cyan => 6,
                colored::Color::White => 7,
                colored::Color::BrightBlack => 8,
                colored::Color::BrightRed => 9,
                colored::Color::BrightGreen => 10,
                colored::Color::BrightYellow => 11,
                colored::Color::BrightBlue => 12,
                colored::Color::BrightMagenta => 13,
                colored::Color::BrightCyan => 14,
                colored::Color::BrightWhite => 15,
                colored::Color::AnsiColor(n) => 16 + n as u32,
                colored::Color::TrueColor { r, g, b } => {
                    17 + u8::MAX as u32 + r as u32 + g as u32 + b as u32
                }
            },
            None => u32::MAX,
        });
    }
}

impl Color {
    fn colorize(&self, text: &str) -> String {
        if let Some(c) = self.color {
            text.color(c).to_string()
        } else {
            text.to_string()
        }
    }
}

macro_rules! define_host_struct {
    ($name:ident, $def:ident ) => {
        #[derive(Deserialize, Clone, Debug, PartialEq, Hash)]
        /// Representation of a repository $def Host.
        pub struct $name {
            /// Name of the remote host.
            pub name: String,
            /// Name of the directory for that host in the repo tree.
            dir_name: Option<String>,
            /// Short representation of the host.
            repr: Option<String>,
            #[serde(default)]
            /// Color for the short representation of the host.
            repr_color: Color,
        }

        impl $name {
            /// Get the directory name for that host in the repo tree.
            pub fn dir_name(&self) -> String {
                self.dir_name.clone().unwrap_or(self.name.clone())
            }

            /// Get the short representation of the host.
            pub fn repr(&self) -> String {
                self.repr_color
                    .colorize(self.repr.as_deref().unwrap_or(&self.name))
            }
        }
    };
}

define_host_struct!(Host, remote);
type Hosts = HashMap<String, Host>;

fn default_hosts() -> Vec<(String, Host)> {
    let msg = "Hardcoded value must be valid";
    [
        (
            "github.com",
            "github",
            "",
            Color::from_str("white").expect(msg),
        ),
        ("gitlab.com", "gitlab", "󰮠", Color::from(166)),
        (
            "git.kernel.org",
            "kernel",
            "",
            Color::from_str("white").expect(msg),
        ),
        (
            "bitbucket.org",
            "bitbucket",
            "",
            Color::from_str("blue").expect(msg),
        ),
        (
            "codeberg.org",
            "codeberg",
            "",
            Color::from_str("blue").expect(msg),
        ),
    ]
    .into_iter()
    .map(|(u, n, r, repr_color)| {
        (
            u.to_string(),
            Host {
                name: n.to_string(),
                dir_name: None,
                repr: Some(r.to_string()),
                repr_color,
            },
        )
    })
    .collect()
}

define_host_struct!(Local, remote);

impl Local {
    pub fn as_host(&self) -> Host {
        Host {
            name: self.name.clone(),
            dir_name: self.dir_name.clone(),
            repr: self.repr.clone(),
            repr_color: self.repr_color.clone(),
        }
    }
}

impl Default for Local {
    fn default() -> Self {
        Self {
            name: "local".to_string(),
            dir_name: None,
            repr: Some("󰋊".to_string()),
            repr_color: Color::from_str("white")
                .expect("Hardcoded value must be valid"),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct CloneCommandConfig {
    #[serde(default)]
    pub vcs: VersionControlSystem,
}

#[derive(Deserialize, Default)]
pub struct ResolveCommandConfig {
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

#[derive(Deserialize, Default)]
pub struct TodoCommandConfig {
    #[serde(default)]
    pub ignore: Vec<String>,
}

#[derive(Deserialize, Default)]
pub struct CommandConfig {
    pub clone: CloneCommandConfig,
    pub resolve: ResolveCommandConfig,
    pub todo: TodoCommandConfig,
}

/// Configuration of the rt executable.
#[derive(Deserialize, Default)]
pub struct Config {
    // Value obtained through environment variable REPO_TREE_DIR.
    /// Path the root of the repo tree.
    #[serde(skip_deserializing)]
    pub repo_tree_dir: PathBuf,
    /// Configuration related to the hosts we know how to organize repositories
    /// which host there remote.
    #[serde(default)]
    pub hosts: Hosts,
    /// Configuration for local only repositories.
    #[serde(default)]
    pub local: Local,
    /// Configuration for the different rt sub-commands.
    #[serde(default)]
    pub command: CommandConfig,
}

impl Config {
    /// Obtain completion candidates for a CLI host argument.
    pub fn host_completer(&self, current: &OsStr) -> Vec<CompletionCandidate> {
        let mut ret: Vec<CompletionCandidate> = self
            .hosts
            .iter()
            .filter(|(host, _)| {
                host.starts_with(current.to_str().unwrap_or(""))
            })
            .map(|(host, data)| {
                CompletionCandidate::new(data.name.clone())
                    .help(Some(StyledStr::from(host)))
            })
            .collect();

        ret.push(
            CompletionCandidate::new(self.local.name.clone())
                .help(Some(StyledStr::from("Local repositories"))),
        );

        ret
    }

    /// Get the specified Host struct for a given host.
    pub fn get_host(&self, host: &str) -> Option<&Host> {
        self.hosts.get(host)
    }
}

impl Config {
    /// Load the configuration.
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let repo_tree_dir = PathBuf::from(
            &env::var("REPO_TREE_DIR")
                .expect("Missing REPO_TREE_DIR environment variable"),
        );

        assert!(
            repo_tree_dir.is_absolute(),
            "REPO_TREE_DIR value must be an absolute path"
        );

        let config_path = std::env::var("XDG_CONFIG_HOME")
            .map_or(
                std::env::var("HOME").map(|x| Path::new(&x).join(".config")),
                |x| Ok(PathBuf::from(x)),
            )?
            .join("repo-tree")
            .join("config.toml");

        let mut ret: Self = if config_path.is_file() {
            toml::from_str(&fs::read_to_string(&config_path)?)?
        } else {
            Self::default()
        };

        ret.repo_tree_dir = repo_tree_dir;

        for (url, host) in default_hosts() {
            if ret.hosts.contains_key(&url) {
                continue;
            }
            ret.hosts.entry(url).or_insert(host);
        }

        Ok(ret)
    }
}

pub fn list_host_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    Config::load().map_or(Vec::new(), |c| c.host_completer(current))
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn empty_config() -> Result<(), Box<dyn Error>> {
        let _: Config = toml::from_str("")?;
        Ok(())
    }

    #[test]
    fn full_config() -> Result<(), Box<dyn Error>> {
        let _: Config = toml::from_str(indoc! {
        r#"
            [hosts."my.custom-domain.fr"]
            name = 'mine'
            repr = '󱘎'
            repr_color = 'blue'

            [hosts."git.buildroot.net"]
            name = 'buildroot'
            dir_name = '.'
            repr = '󰥯'
            repr_color = 'yellow'

            [hosts."busybox.net"]
            name = 'busybox'
            repr = ''
            repr_color = 'green'

            [hosts."blabla.net"]
            name = 'blabla'
            repr = ''
            repr_color = 124

            [local]
            name = 'local'
            repr = '󰋊'
            repr_color = 'white'

            [command.resolve.aliases]
            rt = 'repo-tree'

            [command.todo]
            ignore = [ 'Paluche/jj-test-repo' ]

            [command.clone]
            vcs = 'jujutsu'
            "#
        })?;
        Ok(())
    }
}
