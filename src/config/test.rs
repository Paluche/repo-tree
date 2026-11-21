//! Test of the configuration.

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use super::config::Config;
use super::config_dir_from;

/// Load the configuration `content` with the test `REPO_TREE_DIR`.
fn load(content: &str) -> Result<Config, Box<dyn Error>> {
    Config::load_internal(content, Some(OsString::from("/home/user/work")))
}

/// Serialize the configuration, to compare configurations.
fn serialized(config: &Config) -> String {
    toml::to_string(config).expect("the configuration is serializable")
}

#[test]
fn default_config() -> Result<(), Box<dyn Error>> {
    let config = load("")?;

    // Check the serialized output if the expected one.
    insta::assert_snapshot!(toml::to_string(&config)?, @r#"
    root = "/home/user/work"

    [ui.hint]
    text = "Hint"
    color = "cyan"

    [ui.warning]
    text = "Warning"
    color = 166

    [ui.error]
    text = "Error"
    color = "red"

    [host."bitbucket.org"]
    name = "bitbucket"
    forge = "Bitbucket"

    [host."bitbucket.org".repr]
    text = ""
    color = "blue"

    [host."codeberg.org"]
    name = "codeberg"
    forge = "Forgejo"

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
    forge = "GitHub"

    [host."github.com".repr]
    text = ""
    color = "white"

    [host."gitlab.com"]
    name = "gitlab"
    forge = "GitLab"

    [host."gitlab.com".repr]
    text = "󰮠"
    color = 166

    [tree.dev]
    name = "dev"

    [tree.dev.repr]
    text = ""
    color = "blue"

    [tree.local]
    name = "local"

    [tree.local.repr]
    text = "󰋊"
    color = "white"

    [tree.archive]
    name = "archive"

    [tree.archive.repr]
    text = "󰀼"
    color = "yellow"

    [tree.agent]
    name = "agent"

    [tree.agent.repr]
    text = "󰚩"
    color = "bright yellow"

    [unknown_host.repr]
    text = ""
    color = "red"

    [prompt]
    id = "green"
    workspace = "bright green"

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

    [prompt.git]
    staged = "green"
    unstaged = "red"

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

    [prompt.git.status]
    added = ""
    modified = ""
    file_type_changed = ""
    copied = ""
    renamed = ""
    deleted = ""
    untracked = ""

    [prompt.git.submodule_status]
    color = "red"
    commit_changed = ""
    tracked_changed = ""
    has_untracked = ""

    [prompt.git.stash]
    text = ""
    color = "white"

    [prompt.jj.bookmark]
    local = "bright green"
    tracked = "bright magenta"

    [prompt.jj.bookmark.untracked]
    name = "magenta"
    remote = 125

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

    [prompt.jj.bookmark.deleted]
    prefix = "󰠙"
    separator = "🞍"
    color = 166

    [prompt.jj.tag]
    local = "bright yellow"
    tracked = "yellow"

    [prompt.jj.tag.untracked]
    name = "yellow"
    remote = 94

    [prompt.jj.tag.parent]
    prefix = ""
    separator = "🞍"
    color = "yellow"

    [prompt.jj.tag.current]
    prefix = ""
    separator = "🞍"
    color = "bright blue"

    [prompt.jj.tag.descendants]
    prefix = ""
    separator = "🞍"
    color = "bright blue"

    [prompt.jj.tag.deleted]
    prefix = "󱈠"
    separator = "🞍"
    color = 166

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

    [command.git_status]
    unimportant = "white"
    updated = "red"
    staged = "green"
    unstaged = "red"
    submodule = "blue"

    [command.git_status.upstream]
    name = "cyan"

    [command.git_status.upstream.ahead]
    text = ""
    color = "green"

    [command.git_status.upstream.behind]
    text = ""
    color = "red"

    [command.git_status.entry]
    unmodified = " "
    modified = "M"
    file_type_changed = "T"
    added = "A"
    deleted = "D"
    renamed = "R"
    copied = "C"
    updated = "U"
    untracked = "?"
    ignored = "!"
    "#);

    Ok(())
}

#[test]
fn serialized_default_config_loads_back() -> Result<(), Box<dyn Error>> {
    let config = load("")?;

    assert_eq!(
        serialized(&load(&serialized(&config))?),
        serialized(&config)
    );
    Ok(())
}

/// Collect the path of every configuration section of `table`, prefixed by
/// `prefix`.
///
/// Colored texts, lists and references are values, not sections: they are not
/// partially overridden but replaced.
fn table_paths(table: &toml::Table, prefix: &str, paths: &mut Vec<String>) {
    for (key, value) in table {
        if let toml::Value::Table(table) = value {
            if table.keys().all(|key| {
                ["text", "color", "prefix", "separator", "name", "remote"]
                    .contains(&key.as_str())
            }) {
                continue;
            }
            let path = format!("{prefix}\"{key}\"");
            paths.push(path.clone());
            table_paths(table, &format!("{path}."), paths);
        }
    }
}

#[test]
fn every_table_can_be_partially_overridden() -> Result<(), Box<dyn Error>> {
    let default = serialized(&load("")?);
    let mut paths = Vec::new();
    table_paths(&toml::from_str(&default)?, "", &mut paths);
    // A host has no default values, its name is required.
    paths.retain(|path| !path.starts_with("\"host\""));
    assert!(paths.len() > 20, "{paths:?}");

    for path in paths {
        let config = load(&format!("[{path}]\n"))
            .map_err(|e| format!("empty [{path}] table rejected: {e}"))?;

        assert_eq!(serialized(&config), default, "empty [{path}] table");
    }
    Ok(())
}

#[test]
fn root_from_configuration() -> Result<(), Box<dyn Error>> {
    let config = Config::load_internal("root = '/srv/tree'", None)?;

    assert_eq!(config.root, PathBuf::from("/srv/tree"));
    Ok(())
}

#[test]
fn root_from_configuration_wins_over_repo_tree_dir()
-> Result<(), Box<dyn Error>> {
    assert_eq!(load("root = '/srv/tree'")?.root, PathBuf::from("/srv/tree"));
    Ok(())
}

#[test]
fn relative_root_is_rejected() {
    let error = load("root = 'tree'").err().expect("relative root");

    assert!(
        error.to_string().contains("must be an absolute path"),
        "{error}"
    );
}

#[test]
fn missing_root_without_repo_tree_dir_is_rejected() {
    let error = Config::load_internal("", None).err().expect("missing root");

    assert!(error.to_string().contains("REPO_TREE_DIR"), "{error}");
}

#[test]
fn relative_repo_tree_dir_is_rejected() {
    let error = Config::load_internal("", Some("tree".into()))
        .err()
        .expect("relative REPO_TREE_DIR");

    assert!(error.to_string().contains("REPO_TREE_DIR"), "{error}");
}

#[test]
fn invalid_toml_is_rejected() {
    assert!(load("root = ").is_err());
}

#[test]
fn user_host_is_added_to_default_hosts() -> Result<(), Box<dyn Error>> {
    let config = load(
        "[host.'git.example.com']\nname = 'example'\ndir_name = 'ex'\nforge = \
         'GitLab'\n",
    )?;

    let host = config
        .get_remote_host("git.example.com")
        .expect("configured host");
    assert_eq!(host.category.name, "example");
    assert_eq!(host.category.dir_name(), "ex");
    assert!(config.get_remote_host("github.com").is_some());
    assert!(config.get_remote_host("codeberg.org").is_some());
    Ok(())
}

#[test]
fn user_host_overrides_default_host() -> Result<(), Box<dyn Error>> {
    let config = load("[host.'github.com']\nname = 'gh'\n")?;

    let host = config.get_remote_host("github.com").expect("github host");
    assert_eq!(host.category.name, "gh");
    assert_eq!(host.category.dir_name(), "gh");
    assert!(host.info.forge.is_none());
    Ok(())
}

#[test]
fn host_without_name_is_rejected() {
    assert!(load("[host.'git.example.com']\ndir_name = 'ex'\n").is_err());
}

#[test]
fn unknown_forge_is_rejected() {
    assert!(
        load("[host.'git.example.com']\nname = 'ex'\nforge = 'Gitea'\n")
            .is_err()
    );
}

#[test]
fn should_be_ignored_by_default() -> Result<(), Box<dyn Error>> {
    let config = load("")?;

    assert!(config.should_be_ignored(Path::new("/tmp/repo")));
    assert!(config.should_be_ignored(Path::new("/home/user/.cache/repo")));
    assert!(!config.should_be_ignored(Path::new("/home/user/projects/repo")));
    // Never ignore a repository inside the tree.
    assert!(!config.should_be_ignored(Path::new("/home/user/work/.x/repo")));
    Ok(())
}

#[test]
fn should_be_ignored_with_overridden_ignore() -> Result<(), Box<dyn Error>> {
    let config = load("[repository]\nignore = ['/opt/**']\n")?;

    assert!(config.should_be_ignored(Path::new("/opt/repo")));
    assert!(!config.should_be_ignored(Path::new("/tmp/repo")));
    Ok(())
}

#[test]
fn should_be_ignored_with_extended_ignore() -> Result<(), Box<dyn Error>> {
    let config = load("[repository]\nextend_ignore = ['/opt/**']\n")?;

    assert!(config.should_be_ignored(Path::new("/opt/repo")));
    assert!(config.should_be_ignored(Path::new("/tmp/repo")));
    Ok(())
}

#[test]
fn invalid_ignore_glob_is_rejected() {
    assert!(load("[repository]\nignore = ['a[']\n").is_err());
}

#[test]
fn config_dir_prefers_xdg_config_home() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        config_dir_from(Some("/xdg".into()), Some("/home/user".into()))?,
        PathBuf::from("/xdg/repo-tree")
    );
    Ok(())
}

#[test]
fn config_dir_falls_back_to_home() -> Result<(), Box<dyn Error>> {
    let expected = PathBuf::from("/home/user/.config/repo-tree");

    assert_eq!(config_dir_from(None, Some("/home/user".into()))?, expected);
    // Empty or relative XDG_CONFIG_HOME must be ignored.
    assert_eq!(
        config_dir_from(Some("".into()), Some("/home/user".into()))?,
        expected
    );
    assert_eq!(
        config_dir_from(Some("config".into()), Some("/home/user".into()))?,
        expected
    );
    Ok(())
}

#[test]
fn config_dir_without_home_fails() {
    assert!(config_dir_from(None, None).is_err());
    assert!(config_dir_from(None, Some("".into())).is_err());
}

#[test]
fn tree_space_partial_override() -> Result<(), Box<dyn Error>> {
    let config = load("[tree.archive]\ndir_name = 'old'\n")?;

    assert_eq!(config.tree.archive.category.name, "archive");
    assert_eq!(config.tree.archive.category.dir_name(), "old");
    assert_eq!(
        config.tree.archive.category.repr,
        load("")?.tree.archive.category.repr
    );
    Ok(())
}

#[test]
fn tree_space_renamed() -> Result<(), Box<dyn Error>> {
    let config = load("[tree.dev]\nname = 'active'\n")?;

    assert_eq!(config.tree.dev.category.name, "active");
    assert_eq!(config.tree.dev.category.dir_name(), "active");
    Ok(())
}

/// Names of the host completion candidates for `current`.
fn host_candidates(config: &Config, current: &str) -> Vec<String> {
    let mut candidates: Vec<String> = config
        .host_candidates(current)
        .iter()
        .map(|c| c.get_value().to_string_lossy().into_owned())
        .collect();
    candidates.sort();
    candidates
}

#[test]
fn host_candidates_match_the_host_name() -> Result<(), Box<dyn Error>> {
    let config = load("[host.'git.example.com']\nname = 'example'\n")?;

    assert_eq!(host_candidates(&config, "kern"), ["kernel"]);
    assert_eq!(host_candidates(&config, "git"), ["github", "gitlab"]);
    assert_eq!(host_candidates(&config, "ex"), ["example"]);
    assert_eq!(host_candidates(&config, "x"), Vec::<String>::new());
    assert_eq!(host_candidates(&config, "").len(), 6);
    Ok(())
}
