//! Format of the configuration file.
//! Should be located in `${XDG_CONFIG_HOME}/workspace/config.yml`.
//! If `XDG_CONFIG_HOME` is not set, then we will use the value
//! `${HOME}/.config` in place.

use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::exit,
};
use colored::Colorize;
use yaml_rust2::{Yaml, YamlLoader, yaml::Hash};

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

fn load_config(hosts: &mut Hosts) -> Result<(), Box<dyn Error>> {
    let mut config_path = std::env::var("XDG_CONFIG_HOME").map_or(
        std::env::var("HOME").map(|x| Path::new(&x).join(".config")),
        |x| Ok(PathBuf::from(x)),
    )?;

    config_path.push("workspace");
    config_path.push("config.yml");

    if !config_path.is_file() {
        // No configuration file present.
        return Ok(());
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
        "B: Expecting only entries in the format `string: string`".to_string(),
    ))?;

    for (key, value) in hash {
        let key = String::from(key.as_str().ok_or(ParseError::new(
            &config_path,
            "Expecting configuration keys to be strings".to_string(),
        ))?);

        match key.as_str() {
            "hosts" => parse_hosts(&config_path, hosts, value),
            key => Err(ParseError::new(
                &config_path,
                format!("Unknown key \"{key}\""),
            )),
        }?;
    }

    Ok(())
}

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
        \n    repr: <string>"
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

    let error_msg_prefix = format!("Host \"{url}\": ");
    let format_error = Err(ParseError::new(
        config_path,
        format!(
            "{error_msg_prefix}Expecting \"hosts\" entries in the format
        \n<string>:
        \n    name: <string>
        \n    repr: <string>"
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
            repr: repr.ok_or(ParseError::new(
                config_path,
                format!("{error_msg_prefix}Missing \"repr\" entry"),
            ))?,
        },
    );

    Ok(())
}

pub struct Config {
    pub hosts: Hosts,
    pub local: Host,
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
        ]
        .iter()
        .map(|(u, n, r)| (u.to_string(), n.to_string(), r.to_string()))
        .for_each(|(url, name, repr)| {
            hosts.insert(url, Host { name, repr });
        });

        load_config(&mut hosts)
            .inspect_err(|e| {
                eprintln!("{e}");
                exit(1);
            })
            .unwrap();

        Self {
            hosts,
            local: Host {
                name: "local".to_string(),
                repr: "󰋊".to_string(),
            },
        }
    }
}

impl Config {
    pub fn get_host(&self, host: &str) -> Option<&Host> {
        self.hosts.get(host)
    }
}
