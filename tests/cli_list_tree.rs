//! `rt list` and `rt tree`: list the repositories of the repo-tree.
mod common;

use common::Env;
use common::sorted_lines;
use common::stdout;
use common::write;

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

/// Line shown by `rt tree` under the submodule `name` of `main`, with the
/// commit recorded for it.
fn submodule_commit_line(tree: &str, name: &str) -> String {
    let mut lines = tree.lines();
    lines
        .find(|line| line.ends_with(&format!("── {name}")))
        .and_then(|_| lines.next())
        .unwrap_or_else(|| panic!("no submodule {name} in:\n{tree}"))
        .trim_start_matches([' ', '│'])
        .to_string()
}

#[test]
fn tree_view_shows_how_far_submodules_are_from_the_recorded_commit() {
    let env = Env::new();
    let sub_url = env.bare_remote("owner/sub");
    let main = env.dev_path("owner/main");
    env.git_repo(&main, Some(&Env::remote_url("owner/main")));
    for name in ["synced", "ahead", "behind", "missing"] {
        env.git(&main, &["submodule", "add", "--quiet", &sub_url, name]);
    }
    let behind = main.join("behind");
    write(&behind.join("new"), "new\n");
    env.git(&behind, &["add", "new"]);
    env.git(&behind, &["commit", "--quiet", "-m", "New commit"]);
    env.git(&main, &["add", "behind"]);
    env.git(&main, &["commit", "--quiet", "-m", "Add submodules"]);
    env.git(&behind, &["reset", "--quiet", "--hard", "HEAD~1"]);
    let ahead = main.join("ahead");
    write(&ahead.join("new"), "new\n");
    env.git(&ahead, &["add", "new"]);
    env.git(&ahead, &["commit", "--quiet", "-m", "New commit"]);
    env.git(
        &main,
        &["submodule", "deinit", "--quiet", "--force", "missing"],
    );

    let assert = env.rt().arg("tree").assert().success();

    let tree = stdout(&assert);
    let recorded = |name: &str| {
        env.git(&main, &["rev-parse", &format!("HEAD:{name}")])
            .trim()
            .to_string()
    };
    assert_eq!(submodule_commit_line(&tree, "synced"), recorded("synced"));
    assert_eq!(
        submodule_commit_line(&tree, "ahead"),
        format!("{} 1 ahead, 0 behind", recorded("ahead"))
    );
    assert_eq!(
        submodule_commit_line(&tree, "behind"),
        format!("{} 0 ahead, 1 behind", recorded("behind"))
    );
    assert_eq!(
        submodule_commit_line(&tree, "missing"),
        format!("{} not checked out", recorded("missing"))
    );
}
