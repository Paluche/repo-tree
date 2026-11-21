//! Cache of the repositories of the repo-tree.
mod common;

use common::Env;
use common::stderr;
use common::stdout;

#[test]
fn first_run_builds_the_cache_without_error() {
    let env = Env::new();
    let repo = env.root.join("local/project");
    env.git_repo(&repo, None);

    let assert = env.rt().arg("list").assert().success();

    assert_eq!(stdout(&assert), format!("{}\n", repo.display()));
    assert!(!stderr(&assert).contains("Error"), "{}", stderr(&assert));
    assert!(env.cache_file().is_file());
}

#[test]
fn cache_is_used_until_refreshed() {
    let env = Env::new();
    let first = env.root.join("local/first");
    env.git_repo(&first, None);
    env.rt().arg("list").assert().success();

    let second = env.root.join("local/second");
    env.git_repo(&second, None);

    let assert = env.rt().arg("list").assert().success();
    assert_eq!(stdout(&assert), format!("{}\n", first.display()));
    assert!(
        !stderr(&assert).contains("Refreshing"),
        "{}",
        stderr(&assert)
    );

    let assert = env.rt().args(["list", "-R"]).assert().success();
    assert_eq!(
        common::sorted_lines(&stdout(&assert)),
        [first.display().to_string(), second.display().to_string()]
    );
}

#[test]
fn refresh_cache_command_finds_new_repositories() {
    let env = Env::new();
    env.rt().arg("list").assert().success();
    let repo = env.root.join("local/project");
    env.git_repo(&repo, None);

    env.rt().arg("refresh-cache").assert().success();

    let assert = env.rt().arg("list").assert().success();
    assert_eq!(stdout(&assert), format!("{}\n", repo.display()));
}

#[test]
fn removed_repository_triggers_a_refresh() {
    let env = Env::new();
    let kept = env.root.join("local/kept");
    let removed = env.root.join("local/removed");
    env.git_repo(&kept, None);
    env.git_repo(&removed, None);
    env.rt().arg("list").assert().success();

    std::fs::remove_dir_all(&removed).expect("cannot remove repository");

    let assert = env.rt().arg("list").assert().success();
    assert_eq!(stdout(&assert), format!("{}\n", kept.display()));
    assert!(
        stderr(&assert).contains("Refreshing"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn corrupted_cache_is_rebuilt() {
    let env = Env::new();
    let repo = env.root.join("local/project");
    env.git_repo(&repo, None);
    common::write(&env.cache_file(), "not = [valid");

    let assert = env.rt().arg("list").assert().success();

    assert_eq!(stdout(&assert), format!("{}\n", repo.display()));
    assert!(
        stderr(&assert).contains("Failure to load the cache"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn refresh_cache_reports_write_failures() {
    let env = Env::new();
    // The cache directory cannot be created where a file is.
    common::write(&env.cache_home.join("repo-tree"), "");

    let assert = env.rt().arg("refresh-cache").assert().code(1);

    assert!(
        stderr(&assert).contains("Unable to write the cache"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn empty_directories_are_reported() {
    let env = Env::new();
    std::fs::create_dir_all(env.root.join("dev/example/empty"))
        .expect("cannot create directory");

    let assert = env.rt().arg("refresh-cache").assert().success();

    assert!(
        stderr(&assert).contains(&format!(
            "Empty directory in repo tree: {}",
            env.root.join("dev/example/empty").display()
        )),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn unexpected_tree_space_directory_is_reported() {
    let env = Env::new();
    std::fs::create_dir_all(env.root.join("misc")).expect("cannot create");

    let assert = env.rt().arg("refresh-cache").assert().success();

    assert!(
        stderr(&assert).contains("Unexpected tree-space directory"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn forgetting_a_deleted_workspace_keeps_the_cache_valid() {
    let env = Env::new();
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
    env.rt().arg("list").assert().success();

    std::fs::remove_dir_all(&agent).expect("cannot remove workspace");
    let assert = env.rt().arg("list").assert().success();
    assert!(
        stderr(&assert).contains("Forgot the workspace agent"),
        "{}",
        stderr(&assert)
    );
    assert_eq!(stdout(&assert), format!("{}\n", main.display()));
    assert!(!env.jj(&main, &["workspace", "list"]).contains("agent"));

    let assert = env.rt().arg("list").assert().success();
    assert!(
        !stderr(&assert).contains("Refreshing"),
        "{}",
        stderr(&assert)
    );
}
