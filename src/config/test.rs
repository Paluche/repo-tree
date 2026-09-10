//! Test of the configuration.

use std::env;
use std::error::Error;

use super::config::Config;

#[test]
fn default_config() -> Result<(), Box<dyn Error>> {
    unsafe {
        env::set_var("REPO_TREE_DIR", "/home/user/work");
    }
    let config = Config::load_internal("")?;

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

    [unknown_host.repr]
    text = ""
    color = "red"

    [prompt]
    id = "green"

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
    remote = "magenta"
    tracked = "bright magenta"

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

    [prompt.jj.bookmark.deleted]
    prefix = "󰠙"
    separator = "🞍"
    color = 166

    [prompt.jj.tags]
    name = "yellow"

    [prompt.jj.tags.repr]
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
