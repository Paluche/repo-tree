use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::exit,
};
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

impl Error for ParseError {}

fn parser_assert(
    cond: bool,
    parse_error: ParseError,
) -> Result<(), ParseError> {
    if cond { Ok(()) } else { Err(parse_error) }
}

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

    let config =
        YamlLoader::load_from_str(&fs::read_to_string(&config_path)?)?;

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

pub struct Config {
    hosts: HashMap<String, String>,
}

impl Default for Config {
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

        Self { hosts }
    }
}

impl Config {
    pub fn get_host(&self, host: &str) -> Option<&String> {
        self.hosts.get(host)
    }
}
