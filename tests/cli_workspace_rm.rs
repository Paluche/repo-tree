//! `rt workspace` and `rt rm`: manage the workspaces and the repositories.
mod common;

use std::path::Path;

use common::Env;
use common::stderr;
use common::stdout;

/// Run `rt args` from the directory `dir`, with `input` as standard input.
fn rt_in(
    env: &Env,
    dir: &Path,
    args: &str,
    input: &str,
) -> assert_cmd::assert::Assert {
    env.sh(&format!(
        "cd '{}' && exec \"$RT\" --color never {args}",
        dir.display()
    ))
    .write_stdin(input)
    .assert()
}

/// Path of the agent workspace of the repository `name`.
fn agent_path(env: &Env, name: &str) -> std::path::PathBuf {
    env.root.join("agent/example").join(name)
}

#[test]
fn workspace_add_from_the_repository() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "jujutsu-git");

    rt_in(&env, &repo, "workspace add", "").success();

    let agent = agent_path(&env, "owner/repo");
    assert!(agent.join(".jj").is_dir());
    assert!(env.jj(&repo, &["workspace", "list"]).contains("agent"));
    let assert = env.rt().arg("list").assert().success();
    assert_eq!(
        common::sorted_lines(&stdout(&assert)),
        [agent.display().to_string(), repo.display().to_string(),]
    );
}

#[test]
fn workspace_add_by_identifier() {
    let env = Env::new();
    env.clone("owner/repo", "jujutsu");

    env.rt()
        .args(["workspace", "add", "repo"])
        .assert()
        .success();

    assert!(agent_path(&env, "owner/repo").join(".jj").is_dir());
}

#[test]
fn workspace_add_to_existing_destination_fails() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "jujutsu-git");
    common::write(&agent_path(&env, "owner/repo").join("file"), "");

    let assert = rt_in(&env, &repo, "workspace add", "").code(1);

    assert!(
        stderr(&assert).contains("already exists"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn workspace_add_to_git_repository_fails() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "git");

    let assert = rt_in(&env, &repo, "workspace add", "").code(1);

    assert!(
        stderr(&assert).contains("not implemented"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn workspace_add_to_local_repository_fails() {
    let env = Env::new();
    let repo = env.root.join("local/project");
    env.jj_repo(&repo, None);

    let assert = rt_in(&env, &repo, "workspace add", "").code(1);

    assert!(
        stderr(&assert).contains(
            "Repository project should not be in the agent tree-space"
        ),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn workspace_add_outside_of_a_repository_fails() {
    let env = Env::new();

    let assert = rt_in(&env, &env.outside, "workspace add", "").code(2);

    assert!(
        stderr(&assert).contains("Current directory is not a repository"),
        "{}",
        stderr(&assert)
    );
}

/// Repository `owner/repo` with an agent workspace, and the path of both.
fn repo_with_workspace(env: &Env) -> (std::path::PathBuf, std::path::PathBuf) {
    let repo = env.clone("owner/repo", "jujutsu-git");
    rt_in(env, &repo, "workspace add", "").success();
    (repo, agent_path(env, "owner/repo"))
}

#[test]
fn workspace_rm_asks_for_confirmation() {
    let env = Env::new();
    let (repo, agent) = repo_with_workspace(&env);

    for input in ["n\n", "", "yes\n"] {
        let assert = rt_in(&env, &repo, "workspace rm", input).code(1);
        assert!(stdout(&assert).contains("[y/N]"), "{}", stdout(&assert));
        assert!(stdout(&assert).contains("Aborting."), "{}", stdout(&assert));
        assert!(agent.is_dir(), "{input:?}");
    }

    rt_in(&env, &repo, "workspace rm", "Y\n").success();

    assert!(!agent.exists());
    assert!(!env.root.join("agent/example").exists());
    assert!(env.root.join("agent").is_dir());
    assert!(!env.jj(&repo, &["workspace", "list"]).contains("agent"));
}

#[test]
fn workspace_rm_forced() {
    let env = Env::new();
    let (repo, agent) = repo_with_workspace(&env);

    let assert = rt_in(&env, &repo, "workspace rm --force", "").success();

    assert!(!stdout(&assert).contains("[y/N]"), "{}", stdout(&assert));
    assert!(!agent.exists());
    let assert = env.rt().arg("list").assert().success();
    assert_eq!(stdout(&assert), format!("{}\n", repo.display()));
}

#[test]
fn workspace_rm_without_workspace_fails() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "jujutsu-git");

    let assert = rt_in(&env, &repo, "workspace rm --force", "").code(2);

    assert!(
        stderr(&assert).contains("does not have a workspace"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn rm_asks_for_confirmation() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "git");

    let assert = env
        .rt()
        .args(["rm", "repo"])
        .write_stdin("n\n")
        .assert()
        .code(1);

    assert!(stdout(&assert).contains("Aborting."), "{}", stdout(&assert));
    assert!(repo.join(".git").is_dir());
    assert!(
        stderr(&assert)
            .contains("Unable to check if the repository has unpushed"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn rm_removes_every_workspace() {
    let env = Env::new();
    let (repo, agent) = repo_with_workspace(&env);
    env.clone("owner/other", "git");

    env.rt()
        .args(["rm", "--force", "owner/repo"])
        .assert()
        .success();

    assert!(!repo.exists());
    assert!(!agent.exists());
    assert!(env.dev_path("owner/other/.git").is_dir());
    let assert = env.rt().arg("list").assert().success();
    assert_eq!(
        stdout(&assert),
        format!("{}\n", env.dev_path("owner/other").display())
    );
}

#[test]
fn rm_warns_about_unpushed_commits() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "jujutsu-git");
    env.jj_commit(&repo, "file", "change\n");

    let assert = env
        .rt()
        .args(["rm", "repo"])
        .write_stdin("y\n")
        .assert()
        .success();

    assert!(
        stderr(&assert).contains("The repository has unpushed commits"),
        "{}",
        stderr(&assert)
    );
    assert!(!repo.exists());
}

#[test]
fn rm_unknown_repository_fails() {
    let env = Env::new();
    env.clone("owner/repo", "git");

    let assert = env.rt().args(["rm", "--force", "zzzz"]).assert().code(2);

    assert!(
        stderr(&assert).contains("No repository found matching"),
        "{}",
        stderr(&assert)
    );
    assert!(env.dev_path("owner/repo").is_dir());
}
