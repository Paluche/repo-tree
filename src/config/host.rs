//! The different host configuration.
use std::collections::BTreeMap;
use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;

use crate::colors::ColoredText;
use crate::colors::IsEmpty;
use crate::forge::Forge;

/// Common trait for Host configuration (RemoteHost, LocalHost and UnknownHost).
pub trait HostInfo {
    /// Get the directory name for that host in the repo tree.
    fn dir_name(&self) -> String;

    /// Get the forge the remote is using if one.
    #[allow(dead_code)]
    fn forge(&self) -> Forge {
        Forge::Unknown
    }
}

#[cfg(test)]
pub trait HostInfoRaw {
    /// Get the raw `name` configuration value.
    fn raw_name(&self) -> Option<&String>;

    /// Get the raw `dir_name` configuration value.
    fn raw_dir_name(&self) -> &Option<String>;

    /// Get the raw `repr` configuration value.
    fn raw_repr(&self) -> &ColoredText;
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Hash)]
/// Representation of a repository remote host.
pub struct RemoteHost {
    /// Name of the remote host.
    pub name: String,
    /// Name of the directory for that host in the repo tree.
    dir_name: Option<String>,
    /// Short representation of the host.
    #[serde(default)]
    repr: ColoredText,
    /// Associated forge.
    #[serde(default = "default_forge")]
    forge: Forge,
}

/// Obtain the default forge to add to the configuration if they are not already
/// configured by the user.
fn default_forge() -> Forge {
    Forge::Unknown
}

impl HostInfo for RemoteHost {
    /// Get the directory name for that host in the repo tree.
    fn dir_name(&self) -> String {
        self.dir_name.clone().unwrap_or(self.name.clone())
    }

    fn forge(&self) -> Forge {
        self.forge.clone()
    }
}

impl Display for RemoteHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.repr.is_empty() {
            write!(f, "{}", self.name)
        } else {
            self.repr.fmt(f)
        }
    }
}

#[cfg(test)]
impl HostInfoRaw for RemoteHost {
    fn raw_name(&self) -> Option<&String> {
        Some(&self.name)
    }

    fn raw_dir_name(&self) -> &Option<String> {
        &self.dir_name
    }

    fn raw_repr(&self) -> &ColoredText {
        &self.repr
    }
}

/// A group of host as map indexed by the URL of the host.
pub type RemoteHosts = BTreeMap<String, RemoteHost>;

/// Obtain the default host to add to the configuration if they are not already
/// configured by the user.
pub fn default_remote_hosts() -> RemoteHosts {
    [
        (
            "github.com",
            "github",
            "",
            colored::Color::White,
            Forge::Unknown,
        ),
        (
            "gitlab.com",
            "gitlab",
            "󰮠",
            colored::Color::AnsiColor(166),
            Forge::Unknown,
        ),
        (
            "git.kernel.org",
            "kernel",
            "",
            colored::Color::White,
            Forge::Unknown,
        ),
        (
            "git.kernel.org",
            "kernel",
            "",
            colored::Color::White,
            Forge::Unknown,
        ),
        (
            "bitbucket.org",
            "bitbucket",
            "",
            colored::Color::Blue,
            Forge::Unknown,
        ),
        (
            "codeberg.org",
            "codeberg",
            "",
            colored::Color::Blue,
            Forge::Unknown,
        ),
        (
            "codeberg.org",
            "codeberg",
            "",
            colored::Color::Blue,
            Forge::Unknown,
        ),
    ]
    .into_iter()
    .map(|(url, name, repr_text, repr_color, forge)| {
        (
            url.to_string(),
            RemoteHost {
                name: name.to_string(),
                dir_name: None,
                repr: ColoredText::new(repr_text, repr_color),
                forge,
            },
        )
    })
    .collect()
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Hash)]
/// Representation of a repository local host.
pub struct LocalHost {
    /// Name of the remote host.
    pub name: String,
    /// Name of the directory for that host in the repo tree.
    dir_name: Option<String>,
    /// Short representation of the host.
    #[serde(default)]
    repr: ColoredText,
}

impl HostInfo for LocalHost {
    fn dir_name(&self) -> String {
        self.dir_name.clone().unwrap_or(self.name.clone())
    }
}

impl Display for LocalHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.repr.is_empty() {
            write!(f, "{}", self.name)
        } else {
            self.repr.fmt(f)
        }
    }
}

#[cfg(test)]
impl HostInfoRaw for LocalHost {
    fn raw_name(&self) -> Option<&String> {
        Some(&self.name)
    }

    fn raw_dir_name(&self) -> &Option<String> {
        &self.dir_name
    }

    fn raw_repr(&self) -> &ColoredText {
        &self.repr
    }
}

impl Default for LocalHost {
    fn default() -> Self {
        Self {
            name: "local".to_string(),
            dir_name: None,
            repr: ColoredText::new("󰋊", colored::Color::White),
        }
    }
}

/// Configuration when having to handle an unknown host (unknown from the
/// configuration).
#[derive(Deserialize, Serialize, Hash, PartialEq)]
pub struct UnknownHost {
    /// Short representation to use if the host is unknown.
    repr: ColoredText,
}

impl HostInfo for UnknownHost {
    fn dir_name(&self) -> String {
        #[cfg(test)]
        {
            "".to_string()
        }
        #[cfg(not(test))]
        {
            panic!("Should not happen");
        }
    }

    fn forge(&self) -> Forge {
        Forge::Unknown
    }
}

#[cfg(test)]
impl HostInfoRaw for UnknownHost {
    fn raw_name(&self) -> Option<&String> {
        None
    }

    fn raw_repr(&self) -> &ColoredText {
        &self.repr
    }

    fn raw_dir_name(&self) -> &Option<String> {
        &None
    }
}

impl Display for UnknownHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.repr.fmt(f)
    }
}

impl Default for UnknownHost {
    fn default() -> Self {
        Self {
            repr: ColoredText::new("", colored::Color::Red),
        }
    }
}
