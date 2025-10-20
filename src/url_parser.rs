//! Tools around parsing of repositories URL.
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::exit,
};
use url::Url;
use yaml_rust2::YamlLoader;

#[derive(Debug, Clone)]
struct ParseError {
    path: PathBuf,
    msg: &'static str,
}

impl ParseError {
    fn new(path: &Path, msg: &'static str) -> Self {
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

fn parser_assert(
    cond: bool,
    parse_error: ParseError,
) -> Result<(), ParseError> {
    if cond { Ok(()) } else { Err(parse_error) }
}

impl Error for ParseError {}

fn load_config(
    res: &mut HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    let mut config_path = std::env::var("XDG_CONFIG_HOME").map_or(
        std::env::var("HOME").map(|x| Path::new(&x).join(".config")),
        |x| Ok(PathBuf::from(x)),
    )?;

    config_path.push("workspace");
    config_path.push("remote_hosts.yml");

    if !config_path.is_file() {
        // No configuration file present.
        return Ok(());
    }

    let config = YamlLoader::load_from_str(&fs::read_to_string(&config_path)?)?;

    parser_assert(
        config.len() == 1,
        ParseError::new(
            &config_path,
            "A: Expecting only entries in the format `string: string`",
        ),
    )?;

    let hash = config.first().unwrap().as_hash().ok_or(ParseError::new(
        &config_path,
        "B: Expecting only entries in the format `string: string`",
    ))?;

    for (key, value) in hash {
        res.insert(
            String::from(key.as_str().ok_or(ParseError::new(
                &config_path,
                "C: Expecting only entries in the format `string: string`",
            ))?),
            String::from(value.as_str().ok_or(ParseError::new(
                &config_path,
                "D: Expecting only entries in the format `string: string`",
            ))?),
        );
    }

    Ok(())
}

enum HostWorkDir {
    Missing(String),
    Resolved(String),
}

impl HostWorkDir {
    fn or_else<F>(self, f: F) -> HostWorkDir
    where
        F: FnOnce() -> HostWorkDir,
    {
        match self {
            Self::Missing(_) => f(),
            res => res,
        }
    }

    fn into_option(self) -> Option<String> {
        match self {
            Self::Missing(_) => None,
            Self::Resolved(res) => Some(res),
        }
    }
}

pub struct UrlParser {
    hosts: HashMap<String, String>,
    missing_hosts: Vec<String>,
}

impl Default for UrlParser {
    fn default() -> Self {
        let mut hosts = HashMap::new();

        for (key, value) in [
            ("github.com", "github"),
            ("gitlab.com", "gitlab"),
            ("git.kernel.org", "kernel"),
            ("git.buildroot.net", "."),
            ("bitbucket.org", "bitbucket"),
        ] {
            hosts.insert(String::from(key), String::from(value));
        }

        load_config(&mut hosts)
            .inspect_err(|e| {
                eprintln!("{e}");
                exit(1);
            })
            .unwrap();

        Self {
            hosts,
            missing_hosts: Vec::new(),
        }
    }
}

impl UrlParser {
    fn get_host_work_dir(&self, host: &str) -> HostWorkDir {
        self.hosts.get(host).cloned().map_or(
            HostWorkDir::Missing(String::from(host)),
            HostWorkDir::Resolved,
        )
    }

    fn _parse(
        &self,
        remote_url: Option<&String>,
    ) -> Option<(Option<String>, String)> {
        let url = remote_url?;
        let (_user, url) = if let Some(parse) = url.split_once("@") {
            (Some(parse.0), parse.1)
        } else {
            (None, url.as_str())
        };

        let url = Url::parse(url).ok()?;

        let host_work_dir = self
            .get_host_work_dir(url.host_str()?)
            .or_else(|| self.get_host_work_dir(url.scheme()));

        let path = {
            let ret = url.path().to_owned();
            if let Some(ret) = ret.strip_prefix('/') {
                String::from(ret)
            } else {
                ret
            }
        };

        if let HostWorkDir::Missing(host) = &host_work_dir
            && !self.missing_hosts.contains(host)
        {
            eprintln!("Missing host configuration for {host}");
        }

        if path.ends_with(".git") {
            Some((
                host_work_dir.into_option(),
                path.strip_suffix(".git").unwrap().to_string(),
            ))
        } else {
            Some((host_work_dir.into_option(), path))
        }
    }

    pub fn parse<P: AsRef<Path>>(
        &self,
        remote_url: Option<&String>,
        repo_path: &P,
    ) -> (Option<String>, String) {
        self._parse(remote_url).unwrap_or((
            Some("local".to_string()),
            repo_path
                .as_ref()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        ))
    }
}
