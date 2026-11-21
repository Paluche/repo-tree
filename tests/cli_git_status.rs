//! `rt git status`: concise status of a Git repository.
mod common;

use std::path::Path;

use common::Env;
use common::stderr;
use common::stdout;

/// Status of the repository from `dir`, with the paths of the environment
/// replaced by `<tree>`.
fn status_in(env: &Env, dir: &Path, args: &str) -> String {
    let assert = env
        .sh(&format!(
            "cd '{}' && exec \"$RT\" --color never git status {args}",
            dir.display()
        ))
        .assert()
        .success();
    stdout(&assert).replace(&env.root.display().to_string(), "<tree>")
}

#[test]
fn status_of_modified_repository() {
    let env = Env::new();
    let sub_url = env.bare_remote("owner/sub");
    let repo = env.clone("owner/repo", "git");
    env.git(&repo, &["submodule", "add", "--quiet", &sub_url, "sub"]);
    env.git(&repo, &["commit", "--quiet", "-m", "Add submodule"]);
    common::write(&repo.join("sub/README"), "changed\n");
    common::write(&repo.join("README"), "modified\n");
    common::write(&repo.join("dir/staged file"), "staged\n");
    env.git(&repo, &["add", "dir/staged file"]);
    env.git(&repo, &["mv", "README", "RENAMED"]);
    common::write(&repo.join("untracked"), "untracked\n");
    common::write(&repo.join("stashed"), "stash\n");
    env.git(
        &repo,
        &[
            "stash",
            "push",
            "--quiet",
            "--include-untracked",
            "--",
            "stashed",
        ],
    );

    let root_view = status_in(&env, &repo, "");
    let sub_dir_view = status_in(&env, &repo.join("dir"), "");
    let no_relative = status_in(&env, &repo.join("dir"), "--no-relative-path");

    insta::assert_snapshot!(
        [root_view, sub_dir_view, no_relative].join("=====\n")
    );
}

#[test]
fn status_with_conflict() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "git");
    env.git(&repo, &["checkout", "--quiet", "-b", "other"]);
    common::write(&repo.join("con flict"), "other\n");
    env.git(&repo, &["add", "con flict"]);
    env.git(&repo, &["commit", "--quiet", "-m", "Other"]);
    env.git(&repo, &["checkout", "--quiet", "main"]);
    common::write(&repo.join("con flict"), "main\n");
    env.git(&repo, &["add", "con flict"]);
    env.git(&repo, &["commit", "--quiet", "-m", "Main"]);
    // The merge fails on the conflict.
    env.sh(&format!("cd '{}' && git merge other", repo.display()))
        .assert()
        .failure();

    let status = status_in(&env, &repo, "");

    assert!(status.contains("merge ongoing"), "{status}");
    assert!(status.contains("AA      con flict"), "{status}");
}

#[test]
fn status_of_fresh_repository() {
    let env = Env::new();
    let repo = env.root.join("local/new");
    std::fs::create_dir_all(&repo).expect("cannot create directory");
    env.git(&repo, &["init", "--quiet"]);

    assert_eq!(status_in(&env, &repo, ""), "┊ (initial) -> main\n\n");
}

#[test]
fn status_of_jj_repository_fails() {
    let env = Env::new();
    let repo = env.root.join("local/jj");
    std::fs::create_dir_all(&repo).expect("cannot create directory");
    env.jj(&repo, &["git", "init", "--no-colocate"]);

    let assert = env
        .rt()
        .args(["git", "status", "-r", repo.to_str().expect("UTF-8")])
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("Status not implemented for jj"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn status_of_missing_directory_fails() {
    let env = Env::new();

    let assert = env
        .rt()
        .args(["git", "status", "-r", "/does/not/exist"])
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("No such directory"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn status_outside_of_any_repository_fails() {
    let env = Env::new();

    let assert = env
        .rt()
        .args(["git", "status", "-r", env.outside.to_str().expect("UTF-8")])
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("No repository found"),
        "{}",
        stderr(&assert)
    );
}
