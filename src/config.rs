//! Format of the configuration file.
//! Should be located in `${XDG_CONFIG_HOME}/repo-tree/config.toml`.
//! If `XDG_CONFIG_HOME` is not set, then we will use the value
//! `${HOME}/.config` in place.
//!
//! See repository README for more information.

use core::str::FromStr;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

use clap::builder::StyledStr;
use clap_complete::engine::CompletionCandidate;
use colored::Colorize;
use colored::{self};
use globset::Glob;
use itertools::join;
use serde::Deserialize;
use serde::Serialize;
use serde::ser::SerializeSeq;

use crate::version_control_system::VersionControlSystem;

/// Trait to implement is_empty().
pub trait IsEmpty {
    /// Find out if the struct is to be considered empty.
    fn is_empty(&self) -> bool;
}

impl<T> IsEmpty for T
where
    T: Deref<Target = str>,
{
    fn is_empty(&self) -> bool {
        self.deref().is_empty()
    }
}

/// Color configuration.
#[derive(Default, Clone, Debug, PartialEq)]
struct Color {
    /// Color.
    color: Option<colored::Color>,
}

impl From<colored::Color> for Color {
    fn from(color: colored::Color) -> Self {
        Self { color: Some(color) }
    }
}

impl FromStr for Color {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(colored::Color::from_str(s)?))
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        Self::from(colored::Color::AnsiColor(value))
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::from(colored::Color::TrueColor { r, g, b })
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

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                Ok(Color::from((
                    seq.next_element()?.unwrap(),
                    seq.next_element()?.unwrap(),
                    seq.next_element()?.unwrap(),
                )))
            }

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str(
                    "a string, an integer or an array of 3 elements \
                     representing a color",
                )
            }
        }

        deserializer.deserialize_any(ColorVisitor)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.color {
            Some(c) => match c {
                colored::Color::Black => serializer.serialize_str("black"),
                colored::Color::Red => serializer.serialize_str("red"),
                colored::Color::Green => serializer.serialize_str("green"),
                colored::Color::Yellow => serializer.serialize_str("yellow"),
                colored::Color::Blue => serializer.serialize_str("blue"),
                colored::Color::Magenta => serializer.serialize_str("magenta"),
                colored::Color::Cyan => serializer.serialize_str("cyan"),
                colored::Color::White => serializer.serialize_str("white"),
                colored::Color::BrightBlack => {
                    serializer.serialize_str("bright black")
                }
                colored::Color::BrightRed => {
                    serializer.serialize_str("bright red")
                }
                colored::Color::BrightGreen => {
                    serializer.serialize_str("bright green")
                }
                colored::Color::BrightYellow => {
                    serializer.serialize_str("bright yellow")
                }
                colored::Color::BrightBlue => {
                    serializer.serialize_str("bright blue")
                }
                colored::Color::BrightMagenta => {
                    serializer.serialize_str("bright magenta")
                }
                colored::Color::BrightCyan => {
                    serializer.serialize_str("bright cyan")
                }
                colored::Color::BrightWhite => {
                    serializer.serialize_str("bright white")
                }
                colored::Color::AnsiColor(n) => {
                    serializer.serialize_i64(n.into())
                }
                colored::Color::TrueColor { r, g, b } => {
                    let mut seq = serializer.serialize_seq(Some(3))?;
                    seq.serialize_element(&r)?;
                    seq.serialize_element(&g)?;
                    seq.serialize_element(&b)?;
                    seq.end()
                }
            },
            None => serializer.serialize_none(),
        }
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
    pub fn colorize<T>(&self, text: T) -> String
    where
        T: ToString,
    {
        if let Some(color) = self.color {
            text.to_string().color(color).to_string()
        } else {
            text.to_string()
        }
    }
}

/// Configuration for a colored text.
#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct ColoredText {
    /// Text value.
    text: String,
    /// Color of the text.
    color: Color,
}

impl Deref for ColoredText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

impl IsEmpty for &ColoredText {
    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl ColoredText {
    /// Create a new ColoredText.
    fn new<S, C>(text: S, color: C) -> Self
    where
        S: ToString,
        Color: From<C>,
    {
        Self {
            text: text.to_string(),
            color: Color::from(color),
        }
    }
}

impl Display for ColoredText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.color.colorize(&self.text))
    }
}

/// Configuration to display with colors a list.
#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct ColoredList {
    /// Prefix to write in front of the list when displaying it.
    prefix: String,
    /// Separator text to write in-between two items of the list.
    separator: String,
    /// Color to use to print the list (including the prefix and separators).
    color: Color,
}

impl ColoredList {
    /// Create a new ColoredList.
    fn new<S, C>(prefix: S, separator: S, color: C) -> Self
    where
        S: ToString,
        Color: From<C>,
    {
        Self {
            prefix: prefix.to_string(),
            separator: separator.to_string(),
            color: Color::from(color),
        }
    }

    /// Obtain a displayable struct for your list, which will respect the
    /// associated configuration.
    pub fn display<'config, 'list, T>(
        &'config self,
        list: &'list [T],
    ) -> ColoredListDisplay<'config, 'list, T>
    where
        T: ToString,
    {
        ColoredListDisplay {
            colored_list: self,
            list,
        }
    }
}

/// Displayable struct for ColoredList.
pub struct ColoredListDisplay<'config, 'list, T> {
    /// Configuration to use to display the list.
    colored_list: &'config ColoredList,
    /// List to display.
    list: &'list [T],
}

impl<'config, 'list, T> IsEmpty for ColoredListDisplay<'config, 'list, T> {
    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl<'config, 'list, T> Display for ColoredListDisplay<'config, 'list, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.colored_list.color.colorize(format!(
                "{}{}",
                self.colored_list.prefix,
                join(self.list.iter(), &self.colored_list.separator)
            ))
        )
    }
}

/// Common trait for Host configuration (RemoteHost, LocalHost and UnknownHost).
pub trait HostInfo {
    /// Get the directory name for that host in the repo tree.
    fn dir_name(&self) -> String;
}

#[cfg(test)]
trait HostInfoRaw {
    /// Get the raw `name` configuration value.
    fn raw_name(&self) -> Option<&String>;

    /// Get the raw `dir_name` configuration value.
    fn raw_dir_name(&self) -> &Option<String>;

    /// Get the raw `repr` configuration value.
    fn raw_repr(&self) -> &ColoredText;
}

/// Define a host-like struct, this is here to assure simple that the struct
/// RemoteHost and LocalHost follows the same content and functions.
macro_rules! define_host_struct {
    ($name:ident, $def:ident ) => {
        #[derive(Serialize, Deserialize, Clone, PartialEq, Hash)]
        /// Representation of a repository $def host.
        pub struct $name {
            /// Name of the remote host.
            pub name: String,
            /// Name of the directory for that host in the repo tree.
            dir_name: Option<String>,
            /// Short representation of the host.
            #[serde(default)]
            repr: ColoredText,
        }

        impl HostInfo for $name {
            /// Get the directory name for that host in the repo tree.
            fn dir_name(&self) -> String {
                self.dir_name.clone().unwrap_or(self.name.clone())
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if self.repr.is_empty() {
                    write!(f, "{}", self.name)
                } else {
                    self.repr.fmt(f)
                }
            }
        }

        #[cfg(test)]
        impl HostInfoRaw for $name {
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
    };
}

define_host_struct!(RemoteHost, remote);

/// A group of host as map indexed by the URL of the host.
type RemoteHosts = BTreeMap<String, RemoteHost>;

/// Obtain the default host to add to the configuration if they are not already
/// configured by the user.
fn default_remote_hosts() -> RemoteHosts {
    [
        ("github.com", "github", "", colored::Color::White),
        ("gitlab.com", "gitlab", "󰮠", colored::Color::AnsiColor(166)),
        ("git.kernel.org", "kernel", "", colored::Color::White),
        ("bitbucket.org", "bitbucket", "", colored::Color::Blue),
        ("codeberg.org", "codeberg", "", colored::Color::Blue),
    ]
    .into_iter()
    .map(|(u, n, r, color)| {
        (
            u.to_string(),
            RemoteHost {
                name: n.to_string(),
                dir_name: None,
                repr: ColoredText::new(r, color),
            },
        )
    })
    .collect()
}

define_host_struct!(LocalHost, local);

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

/// Configuration to representing a version control system.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct VcsPromptConfig {
    /// Git Version Control System representation.
    #[serde(default = "VcsPromptConfig::default_git")]
    pub git: ColoredText,
    /// Jujutsu Version Control System representation.
    #[serde(default = "VcsPromptConfig::default_jj")]
    pub jj: ColoredText,
}

#[allow(clippy::missing_docs_in_private_items)]
impl VcsPromptConfig {
    fn default_git() -> ColoredText {
        ColoredText::new("󰊢", 166)
    }

    fn default_jj() -> ColoredText {
        ColoredText::new("", colored::Color::Blue)
    }
}

impl Default for VcsPromptConfig {
    fn default() -> Self {
        Self {
            git: Self::default_git(),
            jj: Self::default_jj(),
        }
    }
}

/// How to display the upstream information.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GitUpstreamConfig {
    /// Representation to display when the upstream associated with the current
    /// branch is gone.
    #[serde(default = "GitUpstreamConfig::default_gone")]
    gone: String,
    /// Representation to display when the current branch is up-to-date with
    /// its associated upstream.
    #[serde(default = "GitUpstreamConfig::default_up_to_date")]
    up_to_date: String,
    /// Representation to display when the current branch is ahead of its
    /// associated upstream.
    #[serde(default = "GitUpstreamConfig::default_ahead")]
    ahead: String,
    /// Representation to display when the current branch is behind of its
    /// associated upstream.
    #[serde(default = "GitUpstreamConfig::default_behind")]
    behind: String,
    /// Representation to display when the current branch diverged from its
    /// associated upstream.
    #[serde(default = "GitUpstreamConfig::default_diverged")]
    diverged: String,
    /// Representation to display when the current branch has no upstream
    /// associated.
    #[serde(default = "GitUpstreamConfig::default_local")]
    local: String,
    /// Representation to display when the current HEAD is detached from any
    /// branches.
    #[serde(default = "GitUpstreamConfig::default_detached")]
    detached: String,
    /// Color to apply on the upstream representation.
    #[serde(default = "GitUpstreamConfig::default_color")]
    color: Color,
}

#[allow(clippy::missing_docs_in_private_items)]
impl GitUpstreamConfig {
    fn default_gone() -> String {
        "".to_string()
    }

    fn default_up_to_date() -> String {
        "".to_string()
    }

    fn default_ahead() -> String {
        "".to_string()
    }

    fn default_behind() -> String {
        "".to_string()
    }

    fn default_diverged() -> String {
        "".to_string()
    }

    fn default_local() -> String {
        "".to_string()
    }

    fn default_detached() -> String {
        "".to_string()
    }

    fn default_color() -> Color {
        Color::from(208)
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn new<S, C>(
        gone: S,
        up_to_date: S,
        ahead: S,
        behind: S,
        diverged: S,
        local: S,
        detached: S,
        color: C,
    ) -> Self
    where
        S: ToString,
        Color: From<C>,
    {
        Self {
            gone: gone.to_string(),
            up_to_date: up_to_date.to_string(),
            ahead: ahead.to_string(),
            behind: behind.to_string(),
            diverged: diverged.to_string(),
            local: local.to_string(),
            detached: detached.to_string(),
            color: Color::from(color),
        }
    }

    pub fn gone(&self) -> String {
        self.color.colorize(&self.gone)
    }

    pub fn up_to_date(&self) -> String {
        self.color.colorize(&self.up_to_date)
    }

    pub fn ahead(&self) -> String {
        self.color.colorize(&self.up_to_date)
    }

    pub fn behind(&self) -> String {
        self.color.colorize(&self.behind)
    }

    pub fn diverged(&self) -> String {
        self.color.colorize(&self.diverged)
    }

    pub fn detached(&self) -> String {
        self.color.colorize(&self.detached)
    }

    pub fn local(&self) -> String {
        self.color.colorize(&self.local)
    }
}

impl Default for GitUpstreamConfig {
    fn default() -> Self {
        Self {
            gone: Self::default_gone(),
            up_to_date: Self::default_up_to_date(),
            ahead: Self::default_ahead(),
            behind: Self::default_behind(),
            diverged: Self::default_diverged(),
            local: Self::default_local(),
            detached: Self::default_detached(),
            color: Self::default_color(),
        }
    }
}

/// Configuration for the Git prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GitPromptConfig {
    /// How to display the list of ongoing operations.
    #[serde(default = "GitPromptConfig::default_ongoing_operations")]
    pub ongoing_operations: ColoredList,
    /// How to display the list of branches you are at.
    #[serde(default = "GitPromptConfig::default_branches")]
    pub branches: ColoredList,
    /// How to display the list of tags you are at.
    #[serde(default = "GitPromptConfig::default_tags")]
    pub tags: ColoredList,
    /// How to display the upstream information.
    #[serde(default)]
    pub upstream: GitUpstreamConfig,
    /// How to display the fact that there are stashed changes.
    #[serde(default = "GitPromptConfig::default_stash")]
    pub stash: ColoredText,
}

#[allow(clippy::missing_docs_in_private_items)]
impl GitPromptConfig {
    fn default_ongoing_operations() -> ColoredList {
        ColoredList::new("⛏", "🞍", colored::Color::Red)
    }

    fn default_branches() -> ColoredList {
        ColoredList::new("󰫍", "🞍", colored::Color::Blue)
    }

    fn default_tags() -> ColoredList {
        ColoredList::new("", "🞍", colored::Color::Yellow)
    }

    fn default_stash() -> ColoredText {
        ColoredText::new("", colored::Color::White)
    }
}

impl Default for GitPromptConfig {
    fn default() -> Self {
        Self {
            ongoing_operations: Self::default_ongoing_operations(),
            branches: Self::default_branches(),
            tags: Self::default_tags(),
            upstream: GitUpstreamConfig::default(),
            stash: Self::default_stash(),
        }
    }
}

/// Configuration to customize the prompt.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Prefix to put in front of the prompt fields.
    #[serde(default = "PromptConfig::default_prefix")]
    pub prefix: ColoredText,
    /// String to use to separate the different fields of the prompt.
    #[serde(default = "PromptConfig::default_separator")]
    pub separator: ColoredText,
    /// Configuration to representing a version control system.
    #[serde(default)]
    pub vcs: VcsPromptConfig,
    /// Configuration relative to the Git prompt.
    pub git: GitPromptConfig,
}

impl PromptConfig {
    /// Default value for `prefix` configuration.
    fn default_prefix() -> ColoredText {
        ColoredText::new("┣━┫", colored::Color::Cyan)
    }

    /// Default value for `separator` configuration.
    fn default_separator() -> ColoredText {
        ColoredText::new("|", colored::Color::Cyan)
    }
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            prefix: Self::default_prefix(),
            separator: Self::default_separator(),
            vcs: VcsPromptConfig::default(),
            git: GitPromptConfig::default(),
        }
    }
}

/// Configuration regarding allowed repository locations.
#[derive(Serialize, Deserialize, Hash, PartialEq)]
pub struct RepositoryLocation {
    /// List of glob patterns, any repositories path matching one of the
    /// defined pattern will be allowed to live outside the repo tree. No
    /// warning message will be printed when the prompt run.
    #[serde(default = "RepositoryLocation::default_ignore")]
    pub ignore: Vec<Glob>,
    /// List of glob pattern to extend the ignore configuration value.
    pub extend_ignore: Vec<Glob>,
}

impl RepositoryLocation {
    /// Default value for the ignore value of the RepositoryLocation struct.
    fn default_ignore() -> Vec<Glob> {
        ["/tmp/**", "**/.*/**"]
            .into_iter()
            .map(|v| {
                Glob::new(v)
                    .expect("Hardcoded values should be valid glob patterns.")
            })
            .collect()
    }

    /// Find out if a repository located at the specified path should be
    /// ignored as being a badly located repository due to not being within the
    /// repo tree.
    fn should_be_ignored(&self, path: &Path) -> bool {
        path.to_str()
            .map(|path| {
                self.ignore
                    .iter()
                    .chain(self.extend_ignore.iter())
                    .any(|glob| glob.compile_matcher().is_match(path))
            })
            .unwrap_or(false)
    }
}

impl Default for RepositoryLocation {
    fn default() -> Self {
        Self {
            ignore: Self::default_ignore(),
            extend_ignore: Vec::new(),
        }
    }
}

/// Configuration for the `rt clone` command.
#[derive(Serialize, Deserialize, Default)]
pub struct CloneCommandConfig {
    /// Default version control system to use to clone a repository in the repo
    /// tree.
    #[serde(default)]
    pub default_vcs: VersionControlSystem,
}

/// Configuration for the `rt resolve` command.
#[derive(Serialize, Deserialize, Default)]
pub struct ResolveCommandConfig {
    /// Resolution aliases.
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
}

/// Configuration for the `rt todo` command.
#[derive(Serialize, Deserialize, Default)]
pub struct TodoCommandConfig {
    /// List of ID of repositories to be ignored by the command.
    #[serde(default)]
    pub ignore: Vec<String>,
}

/// Configuration for `rt` commands.
#[derive(Serialize, Deserialize, Default)]
pub struct CommandConfig {
    /// Configuration for `rt clone`.
    pub clone: CloneCommandConfig,
    /// Configuration for `rt resolve`.
    pub resolve: ResolveCommandConfig,
    /// Configuration for `rt todo`.
    pub todo: TodoCommandConfig,
}

/// Configuration of the rt executable.
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// Path the root of the repo tree. Value obtained through environment
    /// variable REPO_TREE_DIR.
    #[serde(skip)]
    pub repo_tree_dir: PathBuf,
    /// Configuration related to the hosts we know how to organize repositories
    /// which host there remote.
    #[serde(default = "default_remote_hosts", rename = "host")]
    pub remote_hosts: RemoteHosts,
    /// Configuration for local only repositories.
    #[serde(default)]
    pub local: LocalHost,
    /// Configuration when having to handle an unknown host (unknown from the
    /// configuration).
    #[serde(default)]
    pub unknown_host: UnknownHost,
    /// Configuration to customize the prompt.
    #[serde(default)]
    pub prompt: PromptConfig,
    /// Configuration regarding allowed repository location outside the repo
    /// tree.
    #[serde(default)]
    pub repository: RepositoryLocation,
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

    /// Find out if the specified path is to be ignored regarding the
    /// configuration.
    pub fn should_be_ignored(&self, path: &Path) -> bool {
        !path.starts_with(&self.repo_tree_dir)
            && self.repository.should_be_ignored(path)
    }
}

impl Config {
    /// Internal loading of the configuration, from a configuration content.
    fn load_internal(content: &str) -> Result<Self, Box<dyn Error>> {
        let mut ret: Config = toml::from_str(content)?;

        let repo_tree_dir = PathBuf::from(
            &env::var("REPO_TREE_DIR")
                .expect("Missing REPO_TREE_DIR environment variable"),
        );

        assert!(
            repo_tree_dir.is_absolute(),
            "REPO_TREE_DIR value must be an absolute path"
        );

        ret.repo_tree_dir = repo_tree_dir;

        for (url, host) in default_remote_hosts() {
            if ret.remote_hosts.contains_key(&url) {
                continue;
            }
            ret.remote_hosts.entry(url).or_insert(host);
        }

        Ok(ret)
    }

    /// Load the configuration.
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = std::env::var("XDG_CONFIG_HOME")
            .map_or(
                std::env::var("HOME").map(|x| Path::new(&x).join(".config")),
                |x| Ok(PathBuf::from(x)),
            )?
            .join("repo-tree")
            .join("config.toml");

        Ok(if config_path.is_file() {
            Self::load_internal(&fs::read_to_string(&config_path)?)?
        } else {
            Self::load_internal("")?
        })
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

    /// Check that the remote hosts has the expected keys.
    fn check_remote_hosts(config: &Config, expected_keys: &[&str]) {
        for key in config.remote_hosts.keys() {
            assert!(
                expected_keys.iter().find(|v| v == &key).is_some(),
                "Host \"{key}\" not expected"
            );
        }

        for key in expected_keys.iter() {
            assert!(
                config.remote_hosts.keys().find(|v| v == key).is_some(),
                "Missing host \"{key}\""
            );
        }
    }

    struct HostRef {
        name: &'static str,
        raw_dir_name: Option<&'static str>,
        dir_name: &'static str,
        raw_repr: ColoredText,
        repr: String,
    }

    /// Check a struct implementing HostInfo and HostInfoRaw traits.
    fn check_host<H>(id: &str, host: &H, expected: HostRef)
    where
        H: HostInfo + HostInfoRaw + Display,
    {
        if let Some(name) = host.raw_name() {
            assert_eq!(
                name, expected.name,
                "{id} name: {name} != {}",
                expected.name
            );
        }
        let raw_dir_name = host.raw_dir_name();
        let expected_raw_dir_name =
            expected.raw_dir_name.map(|v| v.to_string());
        assert_eq!(
            raw_dir_name, &expected_raw_dir_name,
            "{id} dir_name: {raw_dir_name:?} != {expected_raw_dir_name:?}",
        );
        let dir_name = host.dir_name();
        assert_eq!(
            dir_name, expected.dir_name,
            "{id} dir_name(): {dir_name} != {}",
            expected.dir_name
        );
        let raw_repr = host.raw_repr();
        assert_eq!(
            raw_repr, &expected.raw_repr,
            "{id} repr: {raw_repr:?} != {:?}",
            expected.raw_repr,
        );
        let repr = format!("{}", host);
        assert_eq!(
            repr, expected.repr,
            "{id} repr(): {repr} != {}",
            expected.repr
        );
    }

    /// Check a remote host from the configuration.
    fn check_remote_host(config: &Config, key: &str, expected: HostRef) {
        let remote_host = config.remote_hosts.get(key).unwrap_or_else(|| {
            panic!("Missing expected remote host \"{key}\"")
        });

        check_host(key, remote_host, expected);
    }

    #[test]
    fn default_config() -> Result<(), Box<dyn Error>> {
        let config = Config::load_internal("")?;

        // Check remote (remote hosts) values.
        check_remote_hosts(
            &config,
            &[
                "github.com",
                "gitlab.com",
                "git.kernel.org",
                "bitbucket.org",
                "codeberg.org",
            ],
        );
        check_remote_host(
            &config,
            "github.com",
            HostRef {
                name: "github",
                raw_dir_name: None,
                dir_name: "github",
                raw_repr: ColoredText::new("", colored::Color::White),
                repr: "".white().to_string(),
            },
        );
        check_remote_host(
            &config,
            "gitlab.com",
            HostRef {
                name: "gitlab",
                raw_dir_name: None,
                dir_name: "gitlab",
                raw_repr: ColoredText::new("󰮠", 166),
                repr: "󰮠".ansi_color(166).to_string(),
            },
        );
        check_remote_host(
            &config,
            "git.kernel.org",
            HostRef {
                name: "kernel",
                raw_dir_name: None,
                dir_name: "kernel",
                raw_repr: ColoredText::new("", colored::Color::White),
                repr: "".white().to_string(),
            },
        );
        check_remote_host(
            &config,
            "bitbucket.org",
            HostRef {
                name: "bitbucket",
                raw_dir_name: None,
                dir_name: "bitbucket",
                raw_repr: ColoredText::new("", colored::Color::Blue),
                repr: "".blue().to_string(),
            },
        );
        check_remote_host(
            &config,
            "codeberg.org",
            HostRef {
                name: "codeberg",
                raw_dir_name: None,
                dir_name: "codeberg",
                raw_repr: ColoredText::new("", colored::Color::Blue),
                repr: "".blue().to_string(),
            },
        );

        // Check local.
        check_host(
            "local",
            &config.local,
            HostRef {
                name: "local",
                raw_dir_name: None,
                dir_name: "local",
                raw_repr: ColoredText::new("󰋊", colored::Color::White),
                repr: "󰋊".white().to_string(),
            },
        );

        // Check unknown host.
        check_host(
            "unknown_host",
            &config.unknown_host,
            HostRef {
                name: "",
                raw_dir_name: None,
                dir_name: "",
                raw_repr: ColoredText::new("", colored::Color::Red),
                repr: "".red().to_string(),
            },
        );

        // Check prompt configuration.
        assert_eq!(
            &config.prompt,
            &PromptConfig {
                prefix: ColoredText::new("┣━┫", colored::Color::Cyan),
                separator: ColoredText::new("|", colored::Color::Cyan),
                vcs: VcsPromptConfig {
                    git: ColoredText::new("󰊢", 166),
                    jj: ColoredText::new("", colored::Color::Blue),
                },
                git: GitPromptConfig {
                    ongoing_operations: ColoredList::new(
                        "⛏",
                        "🞍",
                        colored::Color::Red
                    ),
                    branches: ColoredList::new("󰫍", "🞍", colored::Color::Blue),
                    tags: ColoredList::new("", "🞍", colored::Color::Yellow),
                    upstream: GitUpstreamConfig::new(
                        "", "", "", "", "", "", "", 208,
                    ),
                    stash: ColoredText::new("", colored::Color::White),
                },
            },
        );

        // Check repository ignores.
        assert_eq!(
            config.repository.ignore,
            ["/tmp/**", "**/.*/**"]
                .into_iter()
                .map(|v| {
                    Glob::new(v).expect(
                        "Hardcoded values should be valid glob patterns.",
                    )
                })
                .collect::<Vec<Glob>>()
        );
        assert_eq!(config.repository.extend_ignore, Vec::new());

        // Check resolve command configuration.
        assert_eq!(config.command.resolve.aliases, BTreeMap::new());

        // Check todo command configuration.
        assert_eq!(config.command.todo.ignore, Vec::<String>::new());

        // Check clone command configuration.
        assert_eq!(
            config.command.clone.default_vcs,
            VersionControlSystem::JujutsuGit
        );

        // Check the serialized output if the expected one.
        insta::assert_snapshot!(toml::to_string(&config)?, @r#"
        [host."bitbucket.org"]
        name = "bitbucket"

        [host."bitbucket.org".repr]
        text = ""
        color = "blue"

        [host."codeberg.org"]
        name = "codeberg"

        [host."codeberg.org".repr]
        text = ""
        color = "blue"

        [host."git.kernel.org"]
        name = "kernel"

        [host."git.kernel.org".repr]
        text = ""
        color = "white"

        [host."github.com"]
        name = "github"

        [host."github.com".repr]
        text = ""
        color = "white"

        [host."gitlab.com"]
        name = "gitlab"

        [host."gitlab.com".repr]
        text = "󰮠"
        color = 166

        [local]
        name = "local"

        [local.repr]
        text = "󰋊"
        color = "white"

        [unknown_host.repr]
        text = ""
        color = "red"

        [prompt.prefix]
        text = "┣━┫"
        color = "cyan"

        [prompt.separator]
        text = "|"
        color = "cyan"

        [prompt.vcs.git]
        text = "󰊢"
        color = 166

        [prompt.vcs.jj]
        text = ""
        color = "blue"

        [prompt.git.ongoing_operations]
        prefix = "⛏"
        separator = "🞍"
        color = "red"

        [prompt.git.branches]
        prefix = "󰫍"
        separator = "🞍"
        color = "blue"

        [prompt.git.tags]
        prefix = ""
        separator = "🞍"
        color = "yellow"

        [prompt.git.upstream]
        gone = ""
        up_to_date = ""
        ahead = ""
        behind = ""
        diverged = ""
        local = ""
        detached = ""
        color = 208

        [prompt.git.stash]
        text = ""
        color = "white"

        [repository]
        ignore = ["/tmp/**", "**/.*/**"]
        extend_ignore = []

        [command.clone]
        default_vcs = "jujutsu-git"

        [command.resolve.aliases]

        [command.todo]
        ignore = []
        "#);

        Ok(())
    }

    #[test]
    fn full_config() -> Result<(), Box<dyn Error>> {
        let config = Config::load_internal(indoc! {r#"
        [host."my.custom-domain.fr"]
        name = 'mine'
        repr = { text = '󱘎', color = 'blue' }

        [host."git.buildroot.net"]
        name = 'buildroot'
        dir_name = '.'
        repr = { text = '󰥯', color = 'yellow' }

        [host."busybox.net"]
        name = 'busybox'

        [host."blabla.net"]
        name = 'blabla'
        repr = { text = '', color = 124 }

        [host."alice-and-bob.net"]
        name = 'alice-and-bob'
        repr = { text = '',  color = [48, 15, 16]}

        [local]
        name = 'local'
        repr = {text = 'L', color = 'blue'}

        [unknown_host]
        repr = {text = '?', color = 'bright red'}

        [prompt]
        prefix = {text = '|', color = 'blue'}
        separator = {text = '/', color = 'blue'}

        [prompt.vcs]
        git = { text = 'G', color = 167 }
        jj = { text = 'J', color = 'cyan' }

        [prompt.git.ongoing_operations]
        prefix = ''
        separator = ', '
        color = 'blue'

        [prompt.git.branches]
        prefix = 'B'
        separator = ', '
        color = 'yellow'

        [prompt.git.tags]
        prefix = 'T'
        separator = ', '
        color = 'bright yellow'

        [prompt.git.upstream]
        gone = 'G'
        up_to_date = 'V'
        ahead = 'A'
        behind = 'B'
        diverged = 'D'
        local = 'L'
        detached = '_'
        color = 'green'

        [prompt.git.stash]
        text = 'stash'
        color = 'red'

        [command.resolve.aliases]
        rt = 'repo-tree'

        [command.todo]
        ignore = [ 'Paluche/jj-test-repo' ]

        [command.clone]
        default_vcs = 'jujutsu'
        "#
        })?;

        // Check remote (remote hosts) values.
        check_remote_hosts(
            &config,
            &[
                "github.com",
                "gitlab.com",
                "my.custom-domain.fr",
                "git.buildroot.net",
                "busybox.net",
                "bitbucket.org",
                "blabla.net",
                "alice-and-bob.net",
                "codeberg.org",
                "git.kernel.org",
            ],
        );
        check_remote_host(
            &config,
            "github.com",
            HostRef {
                name: "github",
                raw_dir_name: None,
                dir_name: "github",
                raw_repr: ColoredText::new("", colored::Color::White),
                repr: "".white().to_string(),
            },
        );
        check_remote_host(
            &config,
            "gitlab.com",
            HostRef {
                name: "gitlab",
                raw_dir_name: None,
                dir_name: "gitlab",
                raw_repr: ColoredText::new("󰮠", 166),
                repr: "󰮠".ansi_color(166).to_string(),
            },
        );
        check_remote_host(
            &config,
            "my.custom-domain.fr",
            HostRef {
                name: "mine",
                raw_dir_name: None,
                dir_name: "mine",
                raw_repr: ColoredText::new("󱘎", colored::Color::Blue),
                repr: "󱘎".blue().to_string(),
            },
        );
        check_remote_host(
            &config,
            "git.buildroot.net",
            HostRef {
                name: "buildroot",
                raw_dir_name: Some("."),
                dir_name: ".",
                raw_repr: ColoredText::new("󰥯", colored::Color::Yellow),
                repr: "󰥯".yellow().to_string(),
            },
        );
        check_remote_host(
            &config,
            "bitbucket.org",
            HostRef {
                name: "bitbucket",
                raw_dir_name: None,
                dir_name: "bitbucket",
                raw_repr: ColoredText::new("", colored::Color::Blue),
                repr: "".blue().to_string(),
            },
        );
        check_remote_host(
            &config,
            "busybox.net",
            HostRef {
                name: "busybox",
                raw_dir_name: None,
                dir_name: "busybox",
                raw_repr: ColoredText::default(),
                repr: "busybox".to_string(),
            },
        );
        check_remote_host(
            &config,
            "blabla.net",
            HostRef {
                name: "blabla",
                raw_dir_name: None,
                dir_name: "blabla",
                raw_repr: ColoredText::new("", 124),
                repr: "".ansi_color(124).to_string(),
            },
        );
        check_remote_host(
            &config,
            "alice-and-bob.net",
            HostRef {
                name: "alice-and-bob",
                raw_dir_name: None,
                dir_name: "alice-and-bob",
                raw_repr: ColoredText::new("", (48, 15, 16)),
                repr: ""
                    .color(colored::Color::TrueColor {
                        r: 48,
                        g: 15,
                        b: 16,
                    })
                    .to_string(),
            },
        );
        check_remote_host(
            &config,
            "git.kernel.org",
            HostRef {
                name: "kernel",
                raw_dir_name: None,
                dir_name: "kernel",
                raw_repr: ColoredText::new("", colored::Color::White),
                repr: "".white().to_string(),
            },
        );
        check_remote_host(
            &config,
            "codeberg.org",
            HostRef {
                name: "codeberg",
                raw_dir_name: None,
                dir_name: "codeberg",
                raw_repr: ColoredText::new("", colored::Color::Blue),
                repr: "".blue().to_string(),
            },
        );

        // Check local.
        check_host(
            "local",
            &config.local,
            HostRef {
                name: "local",
                raw_dir_name: None,
                dir_name: "local",
                raw_repr: ColoredText::new("L", colored::Color::Blue),
                repr: "L".blue().to_string(),
            },
        );

        // Check unknown host.
        check_host(
            "unknown_host",
            &config.unknown_host,
            HostRef {
                name: "",
                raw_dir_name: None,
                dir_name: "",
                raw_repr: ColoredText::new("?", colored::Color::BrightRed),
                repr: "?".bright_red().to_string(),
            },
        );

        // Check prompt configuration.
        assert_eq!(
            &config.prompt,
            &PromptConfig {
                prefix: ColoredText::new("|", colored::Color::Blue),
                separator: ColoredText::new("/", colored::Color::Blue),
                vcs: VcsPromptConfig {
                    git: ColoredText::new("G", colored::Color::AnsiColor(167)),
                    jj: ColoredText::new("J", colored::Color::Cyan),
                },
                git: GitPromptConfig {
                    ongoing_operations: ColoredList::new(
                        "",
                        ", ",
                        colored::Color::Blue
                    ),
                    branches: ColoredList::new(
                        "B",
                        ", ",
                        colored::Color::Yellow
                    ),
                    tags: ColoredList::new(
                        "T",
                        ", ",
                        colored::Color::BrightYellow
                    ),
                    upstream: GitUpstreamConfig::new(
                        "G",
                        "V",
                        "A",
                        "B",
                        "D",
                        "L",
                        "_",
                        colored::Color::Green,
                    ),
                    stash: ColoredText::new("stash", colored::Color::Red),
                },
            },
        );

        // Check repository ignores.
        assert_eq!(
            config.repository.ignore,
            ["/tmp/**", "**/.*/**"]
                .into_iter()
                .map(|v| {
                    Glob::new(v).expect(
                        "Hardcoded values should be valid glob patterns.",
                    )
                })
                .collect::<Vec<Glob>>()
        );
        assert_eq!(config.repository.extend_ignore, Vec::new());

        // Check resolve command configuration.
        assert_eq!(
            config.command.resolve.aliases,
            BTreeMap::from_iter(
                vec![("rt".to_string(), "repo-tree".to_string())].into_iter()
            )
        );

        // Check todo command configuration.
        assert_eq!(config.command.todo.ignore, vec!["Paluche/jj-test-repo"]);

        // Check clone command configuration.
        assert_eq!(
            config.command.clone.default_vcs,
            VersionControlSystem::Jujutsu
        );

        insta::assert_snapshot!(toml::to_string(&config)?, @r#"
        [host."alice-and-bob.net"]
        name = "alice-and-bob"

        [host."alice-and-bob.net".repr]
        text = ""
        color = [48, 15, 16]

        [host."bitbucket.org"]
        name = "bitbucket"

        [host."bitbucket.org".repr]
        text = ""
        color = "blue"

        [host."blabla.net"]
        name = "blabla"

        [host."blabla.net".repr]
        text = ""
        color = 124

        [host."busybox.net"]
        name = "busybox"

        [host."busybox.net".repr]
        text = ""

        [host."codeberg.org"]
        name = "codeberg"

        [host."codeberg.org".repr]
        text = ""
        color = "blue"

        [host."git.buildroot.net"]
        name = "buildroot"
        dir_name = "."

        [host."git.buildroot.net".repr]
        text = "󰥯"
        color = "yellow"

        [host."git.kernel.org"]
        name = "kernel"

        [host."git.kernel.org".repr]
        text = ""
        color = "white"

        [host."github.com"]
        name = "github"

        [host."github.com".repr]
        text = ""
        color = "white"

        [host."gitlab.com"]
        name = "gitlab"

        [host."gitlab.com".repr]
        text = "󰮠"
        color = 166

        [host."my.custom-domain.fr"]
        name = "mine"

        [host."my.custom-domain.fr".repr]
        text = "󱘎"
        color = "blue"

        [local]
        name = "local"

        [local.repr]
        text = "L"
        color = "blue"

        [unknown_host.repr]
        text = "?"
        color = "bright red"

        [prompt.prefix]
        text = "|"
        color = "blue"

        [prompt.separator]
        text = "/"
        color = "blue"

        [prompt.vcs.git]
        text = "G"
        color = 167

        [prompt.vcs.jj]
        text = "J"
        color = "cyan"

        [prompt.git.ongoing_operations]
        prefix = ""
        separator = ", "
        color = "blue"

        [prompt.git.branches]
        prefix = "B"
        separator = ", "
        color = "yellow"

        [prompt.git.tags]
        prefix = "T"
        separator = ", "
        color = "bright yellow"

        [prompt.git.upstream]
        gone = "G"
        up_to_date = "V"
        ahead = "A"
        behind = "B"
        diverged = "D"
        local = "L"
        detached = "_"
        color = "green"

        [prompt.git.stash]
        text = "stash"
        color = "red"

        [repository]
        ignore = ["/tmp/**", "**/.*/**"]
        extend_ignore = []

        [command.clone]
        default_vcs = "jujutsu"

        [command.resolve.aliases]
        rt = "repo-tree"

        [command.todo]
        ignore = ["Paluche/jj-test-repo"]
        "#);

        Ok(())
    }
}
