//! `rt list` and `rt tree`: list the repositories of the repo-tree.
mod common;

use common::Env;
use common::sorted_lines;
use common::stdout;

/// Environment with a few repositories in each tree-space.
fn tree() -> Env {
    let env = Env::new();
    env.write_config(
        "[host.'other.test']\nname = 'other'\nrepr = { text = 'O' }\n",
    );
    for name in ["owner/rocket", "owner/pocket", "solo/tools"] {
        env.git_repo(&env.dev_path(name), Some(&Env::remote_url(name)));
    }
    env.git_repo(
        &env.root.join("dev/other/x/y"),
        Some("ssh://other.test/x/y.git"),
    );
    env.git_repo(
        &env.root.join("archive/example/owner/old"),
        Some(&Env::remote_url("owner/old")),
    );
    env.git_repo(&env.root.join("local/notes"), None);
    env
}

/// Sorted paths of the repositories listed by `rt list args`, relative to
/// the root.
fn list(env: &Env, args: &[&str]) -> Vec<String> {
    let assert = env.rt().arg("list").args(args).assert().success();
    let root = format!("{}/", env.root.display());
    sorted_lines(&stdout(&assert).replace(&root, ""))
}

#[test]
fn list_every_repository() {
    let env = tree();

    assert_eq!(
        list(&env, &[]),
        [
            "archive/example/owner/old",
            "dev/example/owner/pocket",
            "dev/example/owner/rocket",
            "dev/example/solo/tools",
            "dev/other/x/y",
            "local/notes",
        ]
    );
}

#[test]
fn list_with_filters() {
    let env = tree();

    assert_eq!(list(&env, &["-H", "other"]), ["dev/other/x/y"]);
    assert_eq!(list(&env, &["-H", "local"]), ["local/notes"]);
    assert_eq!(
        list(&env, &["-H", "ex*", "-T", "archive"]),
        ["archive/example/owner/old"]
    );
    assert_eq!(
        list(&env, &["-N", "owner/*", "-N", "notes"]),
        [
            "archive/example/owner/old",
            "dev/example/owner/pocket",
            "dev/example/owner/rocket",
            "local/notes",
        ]
    );
    assert_eq!(list(&env, &["-T", "agent"]), Vec::<String>::new());
}

#[test]
fn list_with_invalid_glob_is_a_usage_error() {
    let env = tree();

    env.rt().args(["list", "-N", "a["]).assert().code(2);
}

#[test]
fn tree_view() {
    let env = tree();

    let assert = env.rt().arg("tree").assert().success();

    let out =
        stdout(&assert).replace(&env.root.display().to_string(), "<root>");
    insta::assert_snapshot!(out);
}

#[test]
fn tree_view_of_one_tree_space() {
    let env = tree();

    let assert = env
        .rt()
        .args(["tree", "--tree", "archive"])
        .assert()
        .success();

    let out =
        stdout(&assert).replace(&env.root.display().to_string(), "<root>");
    insta::assert_snapshot!(out);
}

#[test]
fn tree_view_of_empty_tree() {
    let env = Env::new();

    let assert = env.rt().arg("tree").assert().success();

    assert_eq!(stdout(&assert), format!("{}\n\n", env.root.display()));
}

#[test]
fn tree_refreshes_the_cache_with_the_global_flag() {
    let env = tree();
    env.rt().arg("tree").assert().success();
    env.git_repo(&env.root.join("local/new"), None);

    for args in [&["tree", "-R"][..], &["-R", "tree"]] {
        let assert = env.rt().args(args).assert().success();
        assert!(stdout(&assert).contains("new"), "{args:?}");
    }
}
