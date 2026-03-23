//! Format of the configuration file.
//! Should be located in `${XDG_CONFIG_HOME}/repo-tree/config.yml`.
//! If `XDG_CONFIG_HOME` is not set, then we will use the value
//! `${HOME}/.config` in place.
//!
//! Configuration Yaml file has the following syntax:
//! ```yaml
//! vcs: <VCS>  # Default VCS used to clone repositories
//! hosts:
//!    <URL>:
//!       name: <HOST PRETTY NAME>
//!       dir_name: <HOST DIR NAME IN TREE>
//!       repr: <PROMPT REPRESENTATION>
//!       repr_COLOR: <COLOR FOR PROMPT REPRESENTATION>
//! local:
//!   name: <LOCAL REPOS PRETTY NAME>
//!   dir_name: <LOCAL REPOS DIR NAME IN TREE>
//!   repr: <PROMPT REPRESENTATION>
//!   repr_COLOR: <COLOR FOR PROMPT REPRESENTATION>
//! vcs: <DEFAULT VCS TO USE>
//! repo_aliases:
//!       <alias>: <repo_name>
//! todo:
//!   ignore:
//!      - <repo_name>
//! ```

use core::str::FromStr;
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::exit,
};

use clap::{ValueEnum, builder::StyledStr};
use clap_complete::engine::CompletionCandidate;
use colored::{Color, Colorize};
use indoc::indoc;
use yaml_rust2::{Yaml, YamlLoader, yaml::Hash};

use crate::VersionControlSystem;

fn get_host_format_help(key: &str, host_key: &str) -> String {
    format!(
        indoc! {r#"
            Expecting "{}" entries in the format
            {}:
                name: <string>
                dir_name: <string> (optional defaults to name)
                repr: <string> (optional)
                expr_color: <u8> (color as text or ANSI color number, optional)
            "#},
        key, host_key
    )
}

#[derive(Debug, Clone)]
struct ParseError {
    path: PathBuf,
    msg: String,
}

impl ParseError {
    fn new(path: &Path, msg: String) -> Self {
        Self {
            path: path.to_path_buf(),
            msg,
        }
    }

    fn hosts_error(path: &Path) -> Self {
        Self::new(path, get_host_format_help("hosts", "<host URL (string)>"))
    }

    fn local_host_error(path: &Path) -> Self {
        Self::new(path, get_host_format_help("local", "local"))
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.msg)
    }
}

impl Error for ParseError {}

fn parser_assert(
    cond: bool,
    parse_error: ParseError,
) -> Result<(), ParseError> {
    if cond { Ok(()) } else { Err(parse_error) }
}

#[derive(Clone, Hash, Debug, PartialEq)]
/// Representation of a repository remote Host.
pub struct Host {
    /// Name of the remote host.
    pub name: String,
    /// Name of the directory for that host in the repo tree.
    pub dir_name: String,
    /// Short representation of the host.
    pub repr: String,
}

impl Host {
    fn new(
        name: String,
        dir_name: Option<String>,
        repr: Option<String>,
    ) -> Self {
        let dir_name = dir_name.unwrap_or(name.clone());
        let repr = repr.unwrap_or(name.clone());
        Host {
            name,
            dir_name,
            repr,
        }
    }
}

pub type Hosts = HashMap<String, Host>;

fn parse_hosts(
    config_path: &Path,
    hosts: &mut Hosts,
    value: &Yaml,
) -> Result<(), ParseError> {
    let hash = value.as_hash().ok_or_else(|| {
        ParseError::new(
            config_path,
            "B: Expecting only entries in the format: `string: string`"
                .to_string(),
        )
    })?;

    for (key, value) in hash {
        let Some(url) = key.as_str() else {
            return Err(ParseError::hosts_error(config_path));
        };
        let Some(value) = value.as_hash() else {
            return Err(ParseError::hosts_error(config_path));
        };
        let host = parse_host(value, |s: &str| {
            ParseError::new(
                config_path,
                format!(
                    "Host \"{url}\": {s}.\n{}",
                    get_host_format_help("hosts", "<host URL (string)>")
                ),
            )
        })?;

        hosts.insert(url.to_string(), host);
    }

    Ok(())
}

fn parse_local_host(
    config_path: &Path,
    local: &mut Host,
    value: &Yaml,
) -> Result<(), ParseError> {
    let Some(value) = value.as_hash() else {
        return Err(ParseError::local_host_error(config_path));
    };

    *local = parse_host(value, |s: &str| {
        ParseError::new(
            config_path,
            format!(
                "\"local\" host configuration: {s}.\n{}",
                get_host_format_help("local", "local")
            ),
        )
    })?;
    Ok(())
}

fn parse_host<F: Fn(&str) -> ParseError>(
    value: &Hash,
    parse_error: F,
) -> Result<Host, ParseError> {
    let mut name: Option<String> = None;
    let mut dir_name: Option<String> = None;
    let mut repr: Option<String> = None;
    let mut repr_color: Option<Color> = None;

    for (key, value) in value {
        let Some(key) = key.as_str() else {
            return Err(parse_error("Invalid non-str key"));
        };

        match key {
            "name" => {
                name = Some(match value.as_str() {
                    None => {
                        return Err(parse_error(
                            "Invalid value for \"name\" key",
                        ));
                    }
                    Some(v) => v.to_string(),
                });
            }
            "dir_name" => {
                dir_name = Some(match value.as_str() {
                    None => {
                        return Err(parse_error(
                            "Invalid value for \"dir_name\" key",
                        ));
                    }
                    Some(v) => v.to_string(),
                });
            }
            "repr" => {
                repr = Some(match value.as_str() {
                    None => {
                        return Err(parse_error(
                            "Invalid value for \"repr\" key",
                        ));
                    }
                    Some(v) => v.to_string(),
                });
            }
            "repr_color" => {
                repr_color = Some(match value.as_i64() {
                    Some(v) => match TryInto::<u8>::try_into(v)
                        .ok()
                        .map(Color::AnsiColor)
                    {
                        Some(c) => c,
                        None => {
                            return Err(parse_error(&format!(
                                "Invalid value \"{v}\" for \"repr_color\" key \
                                 as integer, must be between 0 and 255 \
                                 included."
                            )));
                        }
                    },
                    None => match value.as_str() {
                        Some(v) => match Color::from_str(v).ok() {
                            Some(c) => c,
                            None => {
                                return Err(parse_error(&format!(
                                    "Invalid value \"{v}\" for \"repr_color\" \
                                     key as string"
                                )));
                            }
                        },
                        None => {
                            return Err(parse_error(
                                "Invalid value for \"repr_color\" key being \
                                 not an integer nor string",
                            ));
                        }
                    },
                });
            }
            key => {
                return Err(parse_error(&format!("Unknown key \"{key}\"")));
            }
        }
    }

    Ok(Host::new(
        name.ok_or(parse_error("Missing \"name\" entry"))?,
        dir_name,
        repr.map(|r| {
            repr_color.map_or_else(|| r.clone(), |c| r.color(c).to_string())
        }),
    ))
}

fn parse_vcs(
    config_path: &Path,
    value: &Yaml,
) -> Result<VersionControlSystem, ParseError> {
    VersionControlSystem::from_str(
        value.as_str().ok_or(ParseError::new(
            config_path,
            "Invalid value for \"vcs\" key".to_string(),
        ))?,
        true,
    )
    .map_err(|e| {
        ParseError::new(
            config_path,
            format!("Invalid value for \"vcs\" key: {e}"),
        )
    })
}

pub type RepoAliases = HashMap<String, String>;

fn parse_repo_aliases(
    config_path: &Path,
    repo_aliases: &mut RepoAliases,
    value: &Yaml,
) -> Result<(), ParseError> {
    let hash = value.as_hash().ok_or_else(|| {
        ParseError::new(
            config_path,
            "\"repo_aliases\": Expecting only entries in the format: `string: \
             string`"
                .to_string(),
        )
    })?;

    for (key, value) in hash {
        let Some(key) = key.as_str() else {
            return Err(ParseError::new(
                config_path,
                "\"repo_aliases\": Unexpected non-string key".to_string(),
            ));
        };
        let Some(value) = value.as_str() else {
            return Err(ParseError::new(
                config_path,
                format!(
                    "\"repo_aliases\": Unexepected non-string value for key \
                     \"{key}\""
                ),
            ));
        };

        repo_aliases.insert(key.to_string(), value.to_string());
    }
    Ok(())
}

fn parse_todo(
    config_path: &Path,
    todo_ignore: &mut Vec<String>,
    value: &Yaml,
) -> Result<(), ParseError> {
    let hash = value.as_hash().ok_or_else(|| {
        ParseError::new(
            config_path,
            "\"todo\": Expecting only entries in the format: `string: string`"
                .to_string(),
        )
    })?;

    for (key, value) in hash {
        let key = String::from(key.as_str().ok_or(ParseError::new(
            config_path,
            "\"todo\": Unexpected non-string key".to_string(),
        ))?);

        match key.as_str() {
            "ignore" => parse_todo_ignore(config_path, todo_ignore, value)?,
            key => Err(ParseError::new(
                config_path,
                format!("Unknown key \"todo.{key}\""),
            ))?,
        }
    }

    Ok(())
}

fn parse_todo_ignore(
    config_path: &Path,
    todo_ignore: &mut Vec<String>,
    value: &Yaml,
) -> Result<(), ParseError> {
    let list = value.as_vec().ok_or(ParseError::new(
        config_path,
        "\"todo.ignore\": Expecting as list as value".to_string(),
    ))?;

    for item in list {
        todo_ignore.push(String::from(item.as_str().ok_or(ParseError::new(
            config_path,
            "\"todo.ignore\": Unexpected non-string key".to_string(),
        ))?))
    }
    Ok(())
}

/// rt configuration content.
pub struct Config {
    /// Configured known hosts.
    pub hosts: Hosts,
    /// "Host" configuration for local repositories.
    pub local: Host,
    /// Default version control system to use to clone the repositories.
    pub vcs: VersionControlSystem,
    /// List of repository resolution aliases.
    pub repo_aliases: RepoAliases,
    /// List of ID of the repository to not take into account in the todo
    /// commands.
    pub todo_ignore: Vec<String>,
}

impl Config {
    fn load_config(
        hosts: Hosts,
        local: Host,
        vcs: VersionControlSystem,
    ) -> Result<Self, Box<dyn Error>> {
        let mut ret = Self {
            hosts,
            local,
            vcs,
            repo_aliases: HashMap::new(),
            todo_ignore: Vec::new(),
        };

        let config_path = std::env::var("XDG_CONFIG_HOME")
            .map_or(
                std::env::var("HOME").map(|x| Path::new(&x).join(".config")),
                |x| Ok(PathBuf::from(x)),
            )?
            .join("repo-tree")
            .join("config.yml");

        if !config_path.is_file() {
            // No configuration file present.
            return Ok(ret);
        }

        let config =
            YamlLoader::load_from_str(&fs::read_to_string(&config_path)?)?;

        parser_assert(
            config.len() == 1,
            ParseError::new(
                &config_path,
                "A: Expecting only entries in the format `string: string`"
                    .to_string(),
            ),
        )?;

        let hash = config.first().unwrap().as_hash().ok_or(ParseError::new(
            &config_path,
            "B: Expecting only entries in the format `string: string`"
                .to_string(),
        ))?;

        for (key, value) in hash {
            let key = String::from(key.as_str().ok_or(ParseError::new(
                &config_path,
                "Expecting configuration keys to be strings".to_string(),
            ))?);

            match key.as_str() {
                "hosts" => parse_hosts(&config_path, &mut ret.hosts, value)?,
                "local" => {
                    parse_local_host(&config_path, &mut ret.local, value)?
                }
                "vcs" => ret.vcs = parse_vcs(&config_path, value)?,
                "repo_aliases" => parse_repo_aliases(
                    &config_path,
                    &mut ret.repo_aliases,
                    value,
                )?,
                "todo" => {
                    parse_todo(&config_path, &mut ret.todo_ignore, value)?
                }
                key => Err(ParseError::new(
                    &config_path,
                    format!("Unknown key \"{key}\""),
                ))?,
            };
        }

        Ok(ret)
    }

    /// Obtain completion candidates for a CLI host argument.
    pub fn host_completer(
        &self,
        current: &std::ffi::OsStr,
    ) -> Vec<CompletionCandidate> {
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
}

impl Default for Config {
    fn default() -> Self {
        let mut hosts = HashMap::new();

        [
            ("github.com", "github", "".white()),
            ("gitlab.com", "gitlab", "󰮠".ansi_color(166)),
            ("git.kernel.org", "kernel", "".white()),
            ("bitbucket.org", "bitbucket", "".blue()),
            ("codeberg.org", "codeberg", "".blue()),
        ]
        .iter()
        .map(|(u, n, r)| (u.to_string(), n.to_string(), r.to_string()))
        .for_each(|(url, name, repr)| {
            hosts.insert(url, Host::new(name, None, Some(repr)));
        });

        let local = Host::new("local".to_string(), None, Some("󰋊".to_string()));

        Self::load_config(hosts, local, VersionControlSystem::JujutsuGit)
            .inspect_err(|e| {
                eprintln!("{e}");
                exit(1);
            })
            .unwrap()
    }
}

impl Config {
    /// Get the specified Host struct for a given host.
    pub fn get_host(&self, host: &str) -> Option<&Host> {
        self.hosts.get(host)
    }
}

pub fn list_host_completer(
    current: &std::ffi::OsStr,
) -> Vec<CompletionCandidate> {
    Config::default().host_completer(current)
}
