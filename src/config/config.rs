//! The repo-tree configuration.

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::builder::StyledStr;
use clap_complete::engine::CompletionCandidate;
use serde::Deserialize;
use serde::Serialize;

use super::command::CommandConfig;
use super::config_dir;
use super::host::HostInfo;
use super::host::LocalHost;
use super::host::RemoteHost;
use super::host::RemoteHosts;
use super::host::UnknownHost;
use super::host::default_remote_hosts;
use super::prompt::PromptConfig;
use super::repository_location::RepositoryLocation;
use crate::error::ConfigError;

/// Obtain a default value for the repo tree root.
fn default_root() -> PathBuf {
    let repo_tree_dir = PathBuf::from(&env::var("REPO_TREE_DIR").expect(
        "Missing \"root\" in configuration file, and REPO_TREE_DIR \
         environment variable",
    ));

    assert!(
        repo_tree_dir.is_absolute(),
        "REPO_TREE_DIR environment variable value must be an absolute path"
    );

    repo_tree_dir
}

/// Configuration of the rt executable.
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// Path the root of the repo tree. Default value obtained through
    /// the environment variable REPO_TREE_DIR.
    #[serde(default = "default_root")]
    pub root: PathBuf,
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
    /// Internal data, do not modify. This is to have a pointer to a default
    /// instance of HostInfo with the lifetime of the configuration struct.
    /// Could be a const as 'static will be always a bigger lifetime.
    #[serde(skip, default)]
    pub default_host_info: HostInfo,
}

impl Config {
    /// Internal loading of the configuration, from a configuration content.
    fn load_internal(content: &str) -> Result<Self, Box<dyn Error>> {
        let mut ret: Config = toml::from_str(content)?;

        if !ret.root.is_absolute() {
            return Err(Box::new(ConfigError(
                "\"root\" value in configuration file must be an absolute path"
                    .to_string(),
            )));
        }

        // Fill the default remote host configuration if not overridden.
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
        let config_path = config_dir()?.join("config.toml");

        Ok(if config_path.is_file() {
            Self::load_internal(&fs::read_to_string(&config_path)?)?
        } else {
            Self::load_internal("")?
        })
    }

    /// Obtain completion candidates for a CLI host argument.
    pub fn host_completer(&self, current: &OsStr) -> Vec<CompletionCandidate> {
        let mut ret: Vec<CompletionCandidate> = self
            .remote_hosts
            .iter()
            .filter(|(host, _)| {
                host.starts_with(current.to_str().unwrap_or(""))
            })
            .map(|(host, data)| {
                CompletionCandidate::new(data.category.name.clone())
                    .help(Some(StyledStr::from(host)))
            })
            .collect();

        ret.push(
            CompletionCandidate::new(self.local.category.name.clone())
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
        !path.starts_with(&self.root) && self.repository.should_be_ignored(path)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use globset::Glob;
    use indoc::indoc;

    use super::*;
    use crate::colors::ColoredList;
    use crate::colors::ColoredText;
    use crate::config::prompt::GitPromptConfig;
    use crate::config::prompt::GitUpstreamConfig;
    use crate::config::prompt::JujutsuBookmarkConfig;
    use crate::config::prompt::JujutsuPromptConfig;
    use crate::config::prompt::VcsPromptConfig;
    use crate::config::tree_category::TreeCategory;
    use crate::forge::Forge;
    use crate::version_control_system::VersionControlSystem;

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

    struct TreeCategoryRef {
        name: &'static str,
        raw_dir_name: Option<&'static str>,
        dir_name: &'static str,
        repr: ColoredText,
    }

    /// Check the content of a TreeCategory
    fn check_tree_category(
        id: &str,
        value: &TreeCategory,
        expected: &TreeCategoryRef,
    ) {
        {
            // name
            let name = &value.name;
            let expected_name = &expected.name;
            assert_eq!(
                name, expected_name,
                "{id}.name: {name:?} != {expected_name:?}",
            );
        }

        {
            // dir_name
            let raw_dir_name = value.raw_dir_name();
            let expected_raw_dir_name = expected.raw_dir_name;
            assert_eq!(
                raw_dir_name, expected_raw_dir_name,
                "{id}.dir_name: {raw_dir_name:?} != {expected_raw_dir_name:?}",
            );
        }

        {
            // dir_name()
            let dir_name = value.dir_name();
            let expected_dir_name = expected.dir_name;
            assert_eq!(
                dir_name, expected_dir_name,
                "{id}.dir_name(): {dir_name:?} != {expected_dir_name:?}",
            );
        }

        {
            // repr
            let repr = &value.repr;
            let expected_repr = &expected.repr;
            assert_eq!(
                repr, expected_repr,
                "{id}.repr: {repr:?} != {expected_repr:?}",
            );
        }
    }

    fn check_host_info(id: &str, info: &HostInfo, expected: &HostInfo) {
        assert_eq!(info, expected, "{id}: {info:?} != {expected:?}");
    }

    struct RemoteHostRef {
        category: TreeCategoryRef,
        info: HostInfo,
    }

    fn check_remote_host(config: &Config, key: &str, expected: RemoteHostRef) {
        let remote_host = config.remote_hosts.get(key).unwrap_or_else(|| {
            panic!("Missing expected remote host \"{key}\"")
        });
        check_tree_category(
            &format!("config.[{key}].remote_host.category"),
            &remote_host.category,
            &expected.category,
        );
        check_host_info(
            &format!("config[{key}].remote_host.info"),
            &remote_host.info,
            &expected.info,
        );
    }

    struct LocalHostRef {
        category: TreeCategoryRef,
    }

    fn check_local_host(config: &Config, expected: LocalHostRef) {
        check_tree_category(
            "config.local.remote_host.category",
            &config.local.category,
            &expected.category,
        );
    }

    fn check_unknown_host(config: &Config, expected: UnknownHost) {
        assert_eq!(config.unknown_host, expected, "config.unknown_host");
    }

    #[test]
    fn default_config() -> Result<(), Box<dyn Error>> {
        unsafe {
            env::set_var("REPO_TREE_DIR", "/home/user/work");
        }
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
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "github",
                    raw_dir_name: None,
                    dir_name: "github",
                    repr: ColoredText::new("", colored::Color::White),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "gitlab.com",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "gitlab",
                    raw_dir_name: None,
                    dir_name: "gitlab",
                    repr: ColoredText::new("󰮠", 166),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "git.kernel.org",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "kernel",
                    raw_dir_name: None,
                    dir_name: "kernel",
                    repr: ColoredText::new("", colored::Color::White),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "bitbucket.org",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "bitbucket",
                    raw_dir_name: None,
                    dir_name: "bitbucket",
                    repr: ColoredText::new("", colored::Color::Blue),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "codeberg.org",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "codeberg",
                    raw_dir_name: None,
                    dir_name: "codeberg",
                    repr: ColoredText::new("", colored::Color::Blue),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );

        // Check local.
        check_local_host(
            &config,
            LocalHostRef {
                category: TreeCategoryRef {
                    name: "local",
                    raw_dir_name: None,
                    dir_name: "local",
                    repr: ColoredText::new("󰋊", colored::Color::White),
                },
            },
        );

        // Check unknown host.
        check_unknown_host(
            &config,
            UnknownHost {
                repr: ColoredText::new("", colored::Color::Red),
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
                jj: JujutsuPromptConfig {
                    bookmark: JujutsuBookmarkConfig {
                        parent: ColoredList::new(
                            "󰫍",
                            "🞍",
                            colored::Color::Yellow
                        ),
                        current: ColoredList::new(
                            "󰫍",
                            "🞍",
                            colored::Color::BrightBlue
                        ),
                        descendants: ColoredList::new(
                            "󰫎",
                            "🞍",
                            colored::Color::BrightBlue
                        ),
                        none: ColoredText::new(
                            "󰫌",
                            colored::Color::BrightBlack
                        ),
                    },
                    tags: ColoredList::new("", "🞍", colored::Color::Yellow),
                    wc_conflict: ColoredText::new(
                        "󰝧",
                        colored::Color::BrightRed
                    ),
                    conflict: ColoredText::new("󰝧", colored::Color::Red),
                }
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
        root = "/home/user/work"

        [host."bitbucket.org"]
        name = "bitbucket"
        forge = "Unknown"

        [host."bitbucket.org".repr]
        text = ""
        color = "blue"

        [host."codeberg.org"]
        name = "codeberg"
        forge = "Unknown"

        [host."codeberg.org".repr]
        text = ""
        color = "blue"

        [host."git.kernel.org"]
        name = "kernel"
        forge = "Unknown"

        [host."git.kernel.org".repr]
        text = ""
        color = "white"

        [host."github.com"]
        name = "github"
        forge = "Unknown"

        [host."github.com".repr]
        text = ""
        color = "white"

        [host."gitlab.com"]
        name = "gitlab"
        forge = "Unknown"

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

        [prompt.jj.bookmark.parent]
        prefix = "󰫍"
        separator = "🞍"
        color = "yellow"

        [prompt.jj.bookmark.current]
        prefix = "󰫍"
        separator = "🞍"
        color = "bright blue"

        [prompt.jj.bookmark.descendants]
        prefix = "󰫎"
        separator = "🞍"
        color = "bright blue"

        [prompt.jj.bookmark.none]
        text = "󰫌"
        color = "bright black"

        [prompt.jj.tags]
        prefix = ""
        separator = "🞍"
        color = "yellow"

        [prompt.jj.wc_conflict]
        text = "󰝧"
        color = "bright red"

        [prompt.jj.conflict]
        text = "󰝧"
        color = "red"

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
        root = "/home/user/repos"

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

        [prompt.jj.bookmark]
        parent = { prefix = 'P', separator = ', ', color = 'green' }
        current = { prefix = 'C', separator = ', ', color = 'blue' }
        descendants = { prefix = 'D', separator = ', ', color = 'magenta' }
        none = { text = 'N', color = 'white' }

        [prompt.jj]
        tags = { prefix = 'T', separator = ', ', color = 'bright yellow'}
        wc_conflict = { text = '!', color = 'bright blue'}
        conflict = { text = '!', color = 'blue'}

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
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "github",
                    raw_dir_name: None,
                    dir_name: "github",
                    repr: ColoredText::new("", colored::Color::White),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "gitlab.com",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "gitlab",
                    raw_dir_name: None,
                    dir_name: "gitlab",
                    repr: ColoredText::new("󰮠", 166),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "my.custom-domain.fr",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "mine",
                    raw_dir_name: None,
                    dir_name: "mine",
                    repr: ColoredText::new("󱘎", colored::Color::Blue),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "git.buildroot.net",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "buildroot",
                    raw_dir_name: Some("."),
                    dir_name: ".",
                    repr: ColoredText::new("󰥯", colored::Color::Yellow),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "bitbucket.org",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "bitbucket",
                    raw_dir_name: None,
                    dir_name: "bitbucket",
                    repr: ColoredText::new("", colored::Color::Blue),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "busybox.net",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "busybox",
                    raw_dir_name: None,
                    dir_name: "busybox",
                    repr: ColoredText::default(),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "blabla.net",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "blabla",
                    raw_dir_name: None,
                    dir_name: "blabla",
                    repr: ColoredText::new("", 124),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "alice-and-bob.net",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "alice-and-bob",
                    raw_dir_name: None,
                    dir_name: "alice-and-bob",
                    repr: ColoredText::new("", (48, 15, 16)),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "git.kernel.org",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "kernel",
                    raw_dir_name: None,
                    dir_name: "kernel",
                    repr: ColoredText::new("", colored::Color::White),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );
        check_remote_host(
            &config,
            "codeberg.org",
            RemoteHostRef {
                category: TreeCategoryRef {
                    name: "codeberg",
                    raw_dir_name: None,
                    dir_name: "codeberg",
                    repr: ColoredText::new("", colored::Color::Blue),
                },
                info: HostInfo {
                    forge: Forge::Unknown,
                },
            },
        );

        // Check local.
        check_local_host(
            &config,
            LocalHostRef {
                category: TreeCategoryRef {
                    name: "local",
                    raw_dir_name: None,
                    dir_name: "local",
                    repr: ColoredText::new("L", colored::Color::Blue),
                },
            },
        );

        // Check unknown host.
        check_unknown_host(
            &config,
            UnknownHost {
                repr: ColoredText::new("?", colored::Color::BrightRed),
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
                jj: JujutsuPromptConfig {
                    bookmark: JujutsuBookmarkConfig {
                        parent: ColoredList::new(
                            "P",
                            ", ",
                            colored::Color::Green,
                        ),
                        current: ColoredList::new(
                            "C",
                            ", ",
                            colored::Color::Blue,
                        ),
                        descendants: ColoredList::new(
                            "D",
                            ", ",
                            colored::Color::Magenta,
                        ),
                        none: ColoredText::new("N", colored::Color::White),
                    },
                    tags: ColoredList::new(
                        "T",
                        ", ",
                        colored::Color::BrightYellow
                    ),
                    wc_conflict: ColoredText::new(
                        "!",
                        colored::Color::BrightBlue
                    ),
                    conflict: ColoredText::new("!", colored::Color::Blue),
                }
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
        root = "/home/user/repos"

        [host."alice-and-bob.net"]
        name = "alice-and-bob"
        forge = "Unknown"

        [host."alice-and-bob.net".repr]
        text = ""
        color = [48, 15, 16]

        [host."bitbucket.org"]
        name = "bitbucket"
        forge = "Unknown"

        [host."bitbucket.org".repr]
        text = ""
        color = "blue"

        [host."blabla.net"]
        name = "blabla"
        forge = "Unknown"

        [host."blabla.net".repr]
        text = ""
        color = 124

        [host."busybox.net"]
        name = "busybox"
        forge = "Unknown"

        [host."busybox.net".repr]
        text = ""

        [host."codeberg.org"]
        name = "codeberg"
        forge = "Unknown"

        [host."codeberg.org".repr]
        text = ""
        color = "blue"

        [host."git.buildroot.net"]
        name = "buildroot"
        dir_name = "."
        forge = "Unknown"

        [host."git.buildroot.net".repr]
        text = "󰥯"
        color = "yellow"

        [host."git.kernel.org"]
        name = "kernel"
        forge = "Unknown"

        [host."git.kernel.org".repr]
        text = ""
        color = "white"

        [host."github.com"]
        name = "github"
        forge = "Unknown"

        [host."github.com".repr]
        text = ""
        color = "white"

        [host."gitlab.com"]
        name = "gitlab"
        forge = "Unknown"

        [host."gitlab.com".repr]
        text = "󰮠"
        color = 166

        [host."my.custom-domain.fr"]
        name = "mine"
        forge = "Unknown"

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

        [prompt.jj.bookmark.parent]
        prefix = "P"
        separator = ", "
        color = "green"

        [prompt.jj.bookmark.current]
        prefix = "C"
        separator = ", "
        color = "blue"

        [prompt.jj.bookmark.descendants]
        prefix = "D"
        separator = ", "
        color = "magenta"

        [prompt.jj.bookmark.none]
        text = "N"
        color = "white"

        [prompt.jj.tags]
        prefix = "T"
        separator = ", "
        color = "bright yellow"

        [prompt.jj.wc_conflict]
        text = "!"
        color = "bright blue"

        [prompt.jj.conflict]
        text = "!"
        color = "blue"

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
