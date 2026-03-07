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
//!       name: <DIR_NAME>
//!       repr: <PROMPT REPRESENTATION>
//! ```

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
use colored::Colorize;
use yaml_rust2::{Yaml, YamlLoader, yaml::Hash};

use crate::VersionControlSystem;

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

#[derive(Clone, Hash, Debug)]
pub struct Host {
    pub name: String,
    pub repr: String,
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
            "B: Expecting only entries in the format `string: string`"
                .to_string(),
        )
    })?;

    let format_error = Err(ParseError::new(
        config_path,
        "Expecting \"hosts\" entries in the format
        \n<url (string)>:
        \n    name: <string>
        \n    repr: <string> (optional)
        \n    expr_color: <u8> (ANSI color number, optional)"
            .to_string(),
    ));

    for (key, value) in hash {
        parse_host(
            config_path,
            hosts,
            match key.as_str() {
                None => return format_error,
                Some(v) => v,
            },
            match value.as_hash() {
                None => return format_error,
                Some(v) => v,
            },
        )?;
    }

    Ok(())
}

fn parse_host(
    config_path: &Path,
    hosts: &mut Hosts,
    url: &str,
    value: &Hash,
) -> Result<(), ParseError> {
    let mut name: Option<String> = None;
    let mut repr: Option<String> = None;
    let mut repr_color: Option<u8> = None;

    let error_msg_prefix = format!("Host \"{url}\": ");
    let format_error = Err(ParseError::new(
        config_path,
        format!(
            "{error_msg_prefix}Expecting \"hosts\" entries in the format
        \n<string>:
        \n    name: <string>
        \n    repr: <string> (optional)
        \n    expr_color: <int> (ANSI color number, optional)"
        ),
    ));

    for (key, value) in value {
        let key = match key.as_str() {
            None => return format_error,
            Some(key) => key,
        };

        match key {
            "name" => {
                name = Some(match value.as_str() {
                    None => return format_error,
                    Some(v) => v.to_string(),
                });
            }
            "repr" => {
                repr = Some(match value.as_str() {
                    None => return format_error,
                    Some(v) => v.to_string(),
                });
            }
            "repr_color" => {
                repr_color = Some(match value.as_i64().map(|v| v as u8) {
                    None => return format_error,
                    Some(v) => v,
                });
            }
            key => {
                return Err(ParseError::new(
                    config_path,
                    format!("{error_msg_prefix}Unknown key \"{key}\""),
                ));
            }
        }
    }

    hosts.insert(
        url.to_string(),
        Host {
            name: name.ok_or(ParseError::new(
                config_path,
                format!("{error_msg_prefix}Missing \"url\" entry"),
            ))?,
            repr: repr.map_or(
                Err(ParseError::new(
                    config_path,
                    format!("{error_msg_prefix}Missing \"repr\" entry"),
                )),
                |r| {
                    Ok(repr_color.map_or_else(
                        || r.clone(),
                        |c| r.ansi_color(c).to_string(),
                    ))
                },
            )?,
        },
    );

    Ok(())
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

pub struct Config {
    pub hosts: Hosts,
    pub local: Host,
    pub vcs: VersionControlSystem,
}

impl Config {
    fn load_config(
        hosts: Hosts,
        local: Host,
        vcs: VersionControlSystem,
    ) -> Result<Self, Box<dyn Error>> {
        let mut ret = Self { hosts, local, vcs };

        let mut config_path = std::env::var("XDG_CONFIG_HOME").map_or(
            std::env::var("HOME").map(|x| Path::new(&x).join(".config")),
            |x| Ok(PathBuf::from(x)),
        )?;

        config_path.push("repo-tree");
        config_path.push("config.yml");

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
                "vcs" => ret.vcs = parse_vcs(&config_path, value)?,
                key => Err(ParseError::new(
                    &config_path,
                    format!("Unknown key \"{key}\""),
                ))?,
            };
        }

        Ok(ret)
    }

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
            ("git.buildroot.net", ".", "󰥯".yellow()),
            ("bitbucket.org", "bitbucket", "".blue()),
            ("codeberg.org", "codeberg", "".blue()),
        ]
        .iter()
        .map(|(u, n, r)| (u.to_string(), n.to_string(), r.to_string()))
        .for_each(|(url, name, repr)| {
            hosts.insert(url, Host { name, repr });
        });

        let local = Host {
            name: "local".to_string(),
            repr: "󰋊".to_string(),
        };

        Self::load_config(hosts, local, VersionControlSystem::JujutsuGit)
            .inspect_err(|e| {
                eprintln!("{e}");
                exit(1);
            })
            .unwrap()
    }
}

impl Config {
    pub fn get_host(&self, host: &str) -> Option<&Host> {
        self.hosts.get(host)
    }
}
