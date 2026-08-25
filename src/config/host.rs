//! The different host configuration.
use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::tree_category::TreeCategory;
use crate::colors::ColoredText;
use crate::forge::Forge;

/// Information on a host.
#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct HostInfo {
    /// Associated forge.
    #[serde(default)]
    pub forge: Forge,
}

/// Configuration of a remote host.
#[derive(Serialize, Deserialize, Debug)]
pub struct RemoteHost {
    /// Tree category information.
    #[serde(flatten)]
    pub category: TreeCategory,
    /// Host information.
    #[serde(flatten, default)]
    pub info: HostInfo,
}

/// A group of host as map indexed by the URL base of the host.
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
                category: TreeCategory::new(
                    name.to_string(),
                    None,
                    ColoredText::new(repr_text, repr_color),
                ),
                info: HostInfo { forge },
            },
        )
    })
    .collect()
}

/// Configuration when having to handle an unknown host (unknown from the
/// configuration).
#[derive(Deserialize, Serialize, Hash, PartialEq, Debug)]
pub struct UnknownHost {
    /// Short representation to use if the host is unknown.
    #[serde(default = "UnknownHost::default_repr")]
    pub repr: ColoredText,
}

impl UnknownHost {
    /// Default value for UnknownHost.repr.
    fn default_repr() -> ColoredText {
        ColoredText::new("", colored::Color::Red)
    }
}

impl Default for UnknownHost {
    fn default() -> Self {
        Self {
            repr: Self::default_repr(),
        }
    }
}
