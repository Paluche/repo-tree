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
//! [unknown_host]
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
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;

use clap::builder::StyledStr;
use clap_complete::engine::CompletionCandidate;
use colored::Colorize;
use colored::{self};
use serde::Deserialize;

use crate::VersionControlSystem;

/// Color configuration.
#[derive(Default, Clone, Debug, PartialEq)]
struct Color {
    /// Color.
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

impl Color {
    /// True RGB color.
    fn true_color(r: u8, g: u8, b: u8) -> Self {
        Self {
            color: Some(colored::Color::TrueColor { r, g, b }),
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

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut r: Option<u8> = None;
                let mut g: Option<u8> = None;
                let mut b: Option<u8> = None;

                while let Some((key, value)) = map.next_entry::<&str, i64>()? {
                    match key {
                        "r" => {
                            let value = u8::try_from(value).map_err(|_| {
                                serde::de::Error::custom(format!(
                                    "r (red) color value out of range: {value}"
                                ))
                            })?;
                            r = Some(value);
                        }
                        "g" => {
                            let value = u8::try_from(value).map_err(|_| {
                                serde::de::Error::custom(format!(
                                    "g (green) color value out of range: \
                                     {value}"
                                ))
                            })?;
                            g = Some(value);
                        }
                        "b" => {
                            let value = u8::try_from(value).map_err(|_| {
                                serde::de::Error::custom(format!(
                                    "b (blue) color value out of range: \
                                     {value}"
                                ))
                            })?;
                            b = Some(value);
                        }
                        key => {
                            return Err(serde::de::Error::custom(format!(
                                "Unexpected key {key}"
                            )));
                        }
                    }
                }

                let mut msg = String::new();

                if r.is_none() {
                    msg.push('r');
                }
                if g.is_none() {
                    if !msg.is_empty() {
                        msg.push_str(", ")
                    }
                    msg.push('g')
                }
                if b.is_none() {
                    if !msg.is_empty() {
                        msg.push_str(", ")
                    }
                    msg.push('b')
                }

                if msg.is_empty() {
                    Ok(Color::true_color(r.unwrap(), g.unwrap(), b.unwrap()))
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Missing keys: {msg}"
                    )))
                }
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
    /// Colorize the provided text.
    fn colorize(&self, text: &str) -> String {
        if let Some(c) = self.color {
            text.color(c).to_string()
        } else {
            text.to_string()
        }
    }
}

/// Define a host-like struct, this is here to assure simple that the struct
/// RemoteHost and LocalHost follows the same content and functions.
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

define_host_struct!(RemoteHost, remote);

/// A group of host as map indexed by the URL of the host.
type RemoteHosts = HashMap<String, RemoteHost>;

/// Obtain the default host to add to the configuration if they are not already
/// configured by the user.
fn default_remote_hosts() -> Vec<(String, RemoteHost)> {
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
            RemoteHost {
                name: n.to_string(),
                dir_name: None,
                repr: Some(r.to_string()),
                repr_color,
            },
        )
    })
    .collect()
}

define_host_struct!(LocalHost, local);

impl LocalHost {
    /// Clone the LocalHost struct into a RemoteHost.
    pub fn as_host(&self) -> RemoteHost {
        RemoteHost {
            name: self.name.clone(),
            dir_name: self.dir_name.clone(),
            repr: self.repr.clone(),
            repr_color: self.repr_color.clone(),
        }
    }
}

impl Default for LocalHost {
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

/// Configuration when having to handle an unknown host (unknown from the
/// configuration).
#[derive(Deserialize, Hash, PartialEq, Debug)]
pub struct UnknownHost {
    /// Short representation to use is the host is unknown
    repr: String,
    #[serde(default)]
    /// Color for the short representation of the host.
    repr_color: Color,
}

impl UnknownHost {
    /// Get the short representation of the host.
    pub fn repr(&self) -> String {
        self.repr_color.colorize(&self.repr)
    }
}

impl Default for UnknownHost {
    fn default() -> Self {
        Self {
            repr: "".to_string(),
            repr_color: Color::from_str("red")
                .expect("Hardcoded value must be valid"),
        }
    }
}

/// Configuration for the `rt clone` command.
#[derive(Deserialize, Default, Debug)]
pub struct CloneCommandConfig {
    /// Default version control system to use to clone a repository in the repo
    /// tree.
    #[serde(default)]
    pub vcs: VersionControlSystem,
}

/// Configuration for the `rt resolve` command.
#[derive(Deserialize, Default, Debug)]
pub struct ResolveCommandConfig {
    /// Resolution aliases.
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

/// Configuration for the `rt todo` command.
#[derive(Deserialize, Default, Debug)]
pub struct TodoCommandConfig {
    /// List of ID of repositories to be ignored by the command.
    #[serde(default)]
    pub ignore: Vec<String>,
}

/// Configuration for `rt` commands.
#[derive(Deserialize, Default, Debug)]
pub struct CommandConfig {
    /// Configuration for `rt clone`.
    pub clone: CloneCommandConfig,
    /// Configuration for `rt resolve`.
    pub resolve: ResolveCommandConfig,
    /// Configuration for `rt todo`.
    pub todo: TodoCommandConfig,
}

/// Configuration of the rt executable.
#[derive(Deserialize, Default, Debug)]
pub struct Config {
    // Value obtained through environment variable REPO_TREE_DIR.
    /// Path the root of the repo tree.
    #[serde(skip_deserializing)]
    pub repo_tree_dir: PathBuf,
    /// Configuration related to the hosts we know how to organize repositories
    /// which host there remote.
    #[serde(default, rename = "host")]
    pub remote_hosts: RemoteHosts,
    /// Configuration for local only repositories.
    #[serde(default)]
    pub local: LocalHost,
    /// Configuration when having to handle an unknown host (unknown from the
    /// configuration).
    #[serde(default)]
    pub unknown_host: UnknownHost,
    /// Configuration for the different rt sub-commands.
    #[serde(default)]
    pub command: CommandConfig,
}

impl Config {
    /// Obtain completion candidates for a CLI host argument.
    pub fn host_completer(&self, current: &OsStr) -> Vec<CompletionCandidate> {
        let mut ret: Vec<CompletionCandidate> = self
            .remote_hosts
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

    /// Get the specified RemoteHost struct for a given host.
    pub fn get_remote_host(&self, host: &str) -> Option<&RemoteHost> {
        self.remote_hosts.get(host)
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

        for (url, host) in default_remote_hosts() {
            if ret.remote_hosts.contains_key(&url) {
                continue;
            }
            ret.remote_hosts.entry(url).or_insert(host);
        }

        Ok(ret)
    }
}

/// Obtain the auto-completion candidates for a host argument.
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
        let _: Config = toml::from_str(indoc! {r#"
        [host."my.custom-domain.fr"]
        name = 'mine'
        repr = '󱘎'
        repr_color = 'blue'

        [host."git.buildroot.net"]
        name = 'buildroot'
        dir_name = '.'
        repr = '󰥯'
        repr_color = 'yellow'

        [host."busybox.net"]
        name = 'busybox'
        repr = ''
        repr_color = 'green'

        [host."blabla.net"]
        name = 'blabla'
        repr = ''
        repr_color = 124

        [host."alice-and-bob.net"]
        name = 'alice-and-bob'
        repr = ''
        [host."alice-and-bob.net".repr_color]
        r = 48
        g = 15
        b = 16

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
