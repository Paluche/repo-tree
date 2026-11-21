//! `rt resolve` and `rt resolve-url`: find the path of a repository.
mod common;

use std::path::Path;

use common::Env;
use common::stderr;
use common::stdout;

/// Environment with the repositories `owner/rocket`, `owner/pocket`,
/// `other/tools` in dev, `owner/old` in archive and `notes` in local.
fn tree() -> Env {
    let env = Env::new();
    for name in ["owner/rocket", "owner/pocket", "other/tools"] {
        env.git_repo(&env.dev_path(name), Some(&Env::remote_url(name)));
    }
    env.git_repo(
        &env.root.join("archive/example/owner/old"),
        Some(&Env::remote_url("owner/old")),
    );
    env.git_repo(&env.root.join("local/notes"), None);
    // A fake fzf, recording its input and printing $FZF_PICK.
    env.fake_bin(
        "fzf",
        &format!(
            "cat > {}\nprintf '%s\\n' \"$FZF_PICK\"",
            env.outside.join("fzf-input").display()
        ),
    );
    env
}

/// Resolve `args` in `env`, expecting `code`, returning the standard output.
fn resolve(env: &Env, args: &[&str], code: i32) -> (String, String) {
    let assert = env.rt().arg("resolve").args(args).assert().code(code);
    (stdout(&assert), stderr(&assert))
}

/// Path of `path` followed by a new line, as printed by resolve.
fn line(path: &Path) -> String {
    format!("{}\n", path.display())
}

#[test]
fn resolve_by_identifiers() {
    let env = tree();
    let rocket = line(&env.dev_path("owner/rocket"));

    for id in ["rocket", "owner/rocket", "example/owner/rocket"] {
        assert_eq!(resolve(&env, &[id], 0).0, rocket, "{id}");
    }
    assert_eq!(
        resolve(&env, &["notes"], 0).0,
        line(&env.root.join("local/notes"))
    );
    assert_eq!(
        resolve(&env, &["old"], 0).0,
        line(&env.root.join("archive/example/owner/old"))
    );
}

#[test]
fn resolve_single_fuzzy_match() {
    let env = tree();

    let (out, err) = resolve(&env, &["tls"], 0);

    assert_eq!(out, line(&env.dev_path("other/tools")));
    assert!(err.contains("Considering you meant"), "{err}");
}

#[test]
fn resolve_ambiguous_identifier() {
    let env = tree();

    let (out, err) = resolve(&env, &["ocket"], 2);

    assert_eq!(out, "");
    assert!(err.contains("Several possible match"), "{err}");
    assert!(err.contains("rocket") && err.contains("pocket"), "{err}");
}

#[test]
fn resolve_unknown_identifier() {
    let env = tree();

    let (out, err) = resolve(&env, &["zzzz"], 2);

    assert_eq!(out, "");
    assert!(err.contains("No match for zzzz"), "{err}");
}

#[test]
fn resolve_with_tree_space_filter() {
    let env = tree();

    assert_eq!(resolve(&env, &["--tree", "archive", "rocket"], 2).0, "");
    assert_eq!(
        resolve(&env, &["--tree", "local", "notes"], 0).0,
        line(&env.root.join("local/notes"))
    );
    assert_eq!(
        resolve(&env, &["--kind", "main", "rocket"], 0).0,
        line(&env.dev_path("owner/rocket"))
    );
}

#[test]
fn resolve_with_conflicting_filters_is_a_usage_error() {
    let env = tree();

    resolve(&env, &["--tree", "dev", "--kind", "main", "rocket"], 2);
}

#[test]
fn resolve_alias() {
    let env = tree();
    env.write_config(
        "[command.resolve.aliases]\nr = 'owner/rocket'\nx = 'nothing'\n",
    );

    let (out, err) = resolve(&env, &["r"], 0);

    assert_eq!(out, line(&env.dev_path("owner/rocket")));
    assert!(
        err.contains("Configured alias \"x\" => \"nothing\""),
        "{err}"
    );
}

#[test]
fn resolve_in_empty_tree() {
    let env = Env::new();

    let (out, err) = resolve(&env, &["x"], 2);

    assert_eq!(out, "");
    assert!(err.contains("No repository in repo-tree"), "{err}");
}

#[test]
fn resolve_without_identifier_asks_fzf() {
    let env = tree();

    let assert = env
        .rt()
        .env("FZF_PICK", "pocket")
        .arg("resolve")
        .assert()
        .success();

    assert_eq!(stdout(&assert), line(&env.dev_path("owner/pocket")));
    let input = std::fs::read_to_string(env.outside.join("fzf-input"))
        .expect("fzf was run");
    assert!(
        input.lines().any(|l| l == "example/owner/rocket"),
        "{input}"
    );
}

#[test]
fn resolve_with_nothing_selected() {
    let env = tree();

    let assert = env.rt().env("FZF_PICK", "").arg("resolve").assert().code(2);

    assert!(
        stderr(&assert).contains("Nothing selected"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn resolve_without_fzf_fails() {
    let env = tree();
    std::fs::remove_file(env.bin.join("fzf")).expect("cannot remove fzf");

    // Only keep the directories of git and jj in PATH, there may be a real fzf
    // elsewhere.
    let assert = env
        .rt()
        .env("PATH", &env.bin)
        .arg("resolve")
        .assert()
        .code(1);

    assert_eq!(stdout(&assert), "");
}

/// Environment with the jj repository `owner/repo` in dev and its agent
/// workspace.
fn tree_with_workspace() -> Env {
    let env = tree();
    let main = env.dev_path("owner/repo");
    env.jj_repo(&main, Some(&Env::remote_url("owner/repo")));
    let agent = env.root.join("agent/example/owner/repo");
    std::fs::create_dir_all(agent.parent().expect("has a parent"))
        .expect("cannot create directory");
    env.jj(
        &main,
        &[
            "workspace",
            "add",
            "--name",
            "agent",
            agent.to_str().expect("UTF-8 path"),
        ],
    );
    env
}

#[test]
fn resolve_repository_with_workspaces_asks_the_tree_space() {
    let env = tree_with_workspace();

    let assert = env
        .rt()
        .env("FZF_PICK", "agent")
        .args(["resolve", "owner/repo"])
        .assert()
        .success();
    assert_eq!(
        stdout(&assert),
        line(&env.root.join("agent/example/owner/repo"))
    );

    assert_eq!(
        resolve(&env, &["--kind", "workspace", "owner/repo"], 0).0,
        line(&env.root.join("agent/example/owner/repo"))
    );
    assert_eq!(
        resolve(&env, &["--tree", "dev", "owner/repo"], 0).0,
        line(&env.dev_path("owner/repo"))
    );
}

#[test]
fn resolve_repository_with_workspaces_and_unknown_tree_space_fails() {
    let env = tree_with_workspace();

    let assert = env
        .rt()
        .env("FZF_PICK", "garbage")
        .args(["resolve", "owner/repo"])
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("Unknown tree-space \"garbage\""),
        "{}",
        stderr(&assert)
    );
}

/// Resolve the URL with `args` in `env`, expecting `code`.
fn resolve_url(env: &Env, args: &[&str], code: i32) -> (String, String) {
    let assert = env.rt().arg("resolve-url").args(args).assert().code(code);
    (stdout(&assert), stderr(&assert))
}

#[test]
fn resolve_url_exact_and_fuzzy() {
    let env = tree();

    assert_eq!(
        resolve_url(&env, &[&Env::remote_url("owner/rocket")], 0).0,
        line(&env.dev_path("owner/rocket"))
    );
    let (out, err) = resolve_url(&env, &["tools"], 0);
    assert_eq!(out, line(&env.dev_path("other/tools")));
    assert!(err.contains("Considering you meant"), "{err}");
    let (out, err) = resolve_url(&env, &["ocket"], 2);
    assert_eq!(out, "");
    assert!(err.contains("Several possible match"), "{err}");
    let (out, err) = resolve_url(&env, &["zzzz"], 2);
    assert_eq!(out, "");
    assert!(err.contains("No match for zzzz"), "{err}");
}

#[test]
fn resolve_url_of_repository_with_workspaces() {
    let env = tree_with_workspace();
    let url = Env::remote_url("owner/repo");

    assert_eq!(
        resolve_url(&env, &[&url], 0).0,
        line(&env.dev_path("owner/repo"))
    );
    assert_eq!(
        resolve_url(&env, &["--tree", "dev", &url], 0).0,
        line(&env.dev_path("owner/repo"))
    );
    assert_eq!(
        resolve_url(&env, &["--tree", "agent", &url], 0).0,
        line(&env.root.join("agent/example/owner/repo"))
    );
}
