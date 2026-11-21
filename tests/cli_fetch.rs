//! `rt fetch`: fetch every repository of the repo-tree.
mod common;

use common::Env;
use common::stderr;
use common::stdout;

/// Push a new commit on the remote `name`, returning its commit ID.
fn push_commit(env: &Env, name: &str) -> String {
    let work = env.outside.join(format!("push-{}", name.replace('/', "-")));
    env.git(
        &env.outside,
        &[
            "clone",
            "--quiet",
            &Env::remote_url(name),
            work.to_str().expect("UTF-8 path"),
        ],
    );
    common::write(&work.join("new"), "new\n");
    env.git(&work, &["add", "new"]);
    env.git(&work, &["commit", "--quiet", "-m", "New commit"]);
    env.git(&work, &["push", "--quiet", "origin", "main"]);
    env.git(&work, &["rev-parse", "HEAD"]).trim().to_string()
}

#[test]
fn fetch_every_repository() {
    let env = Env::new();
    env.git_repo(&env.root.join("local/notes"), None);
    let git = env.clone("owner/git", "git");
    let jj = env.clone("owner/jj", "jujutsu-git");
    let git_commit = push_commit(&env, "owner/git");
    let jj_commit = push_commit(&env, "owner/jj");

    let assert = env.rt().arg("fetch").assert().success();

    assert_eq!(
        stdout(&assert).lines().last(),
        Some("2/2 repository fetched")
    );
    assert!(
        stderr(&assert).contains("Skipping local repository notes"),
        "{}",
        stderr(&assert)
    );
    assert_eq!(
        env.git(&git, &["rev-parse", "origin/main"]).trim(),
        git_commit
    );
    assert!(
        env.jj(
            &jj,
            &["log", "--no-graph", "-r", "main@origin", "-T", "commit_id"]
        )
        .starts_with(&jj_commit)
    );
}

#[test]
fn fetch_quiet_prints_only_the_summary() {
    let env = Env::new();
    env.clone("owner/git", "git");

    let assert = env.rt().args(["fetch", "--quiet"]).assert().success();

    assert_eq!(stdout(&assert), "1/1 repository fetched\n");
}

#[test]
fn fetch_submodules() {
    let env = Env::new();
    let sub_url = env.bare_remote("owner/sub");
    let main_url = env.bare_remote("owner/main");
    let work = env.outside.join("work");
    env.git(&env.outside, &["clone", "--quiet", &main_url, "work"]);
    env.git(&work, &["submodule", "add", "--quiet", &sub_url, "sub"]);
    env.git(&work, &["commit", "--quiet", "-m", "Add submodule"]);
    env.git(&work, &["push", "--quiet", "origin", "main"]);
    env.rt()
        .args(["clone", "--vcs", "git", &main_url])
        .assert()
        .success();
    let main = env.dev_path("owner/main");
    let sub_commit = push_commit(&env, "owner/sub");

    let assert = env.rt().arg("fetch").assert().success();

    assert_eq!(
        stdout(&assert).lines().last(),
        Some("2/2 repository fetched")
    );
    assert!(
        stdout(&assert).contains("Fetching git submodule repository"),
        "{}",
        stdout(&assert)
    );
    assert_eq!(
        env.git(&main.join("sub"), &["rev-parse", "origin/main"])
            .trim(),
        sub_commit
    );
}

#[test]
fn fetch_failure_is_reported() {
    let env = Env::new();
    env.clone("owner/ok", "git");
    env.clone("owner/gone", "git");
    env.clone("owner/jj-gone", "jujutsu-git");
    for name in ["owner/gone", "owner/jj-gone"] {
        std::fs::remove_dir_all(env.remotes.join(format!("{name}.git")))
            .expect("cannot remove remote");
    }

    let assert = env.rt().arg("fetch").assert().code(1);

    assert_eq!(
        stdout(&assert).lines().last(),
        Some("1/3 repository fetched")
    );
    let err = stderr(&assert);
    assert!(err.contains("Unable to fetch example owner/gone"), "{err}");
    assert!(
        err.contains("Unable to fetch example owner/jj-gone"),
        "{err}"
    );
}
