//! `rt todo`: find the repositories where something is left to do.
mod common;

use common::Env;
use common::stderr;
use common::stdout;

/// Environment with jj repositories in each state, a Git repository and a
/// local one.
fn tree() -> Env {
    let env = Env::new();
    env.write_config("[command.todo]\nignore = ['owner/ignored']\n");
    env.git_repo(&env.root.join("local/notes"), None);
    env.clone("owner/git", "git");
    env.clone("owner/ignored", "jujutsu-git");
    env.clone("owner/ok", "jujutsu-git");
    let unpushed = env.clone("owner/unpushed", "jujutsu-git");
    env.jj_commit(&unpushed, "file", "change\n");
    let conflicted = env.clone("owner/conflicted", "jujutsu-git");
    env.jj_conflict(&conflicted);
    env.rt().arg("refresh-cache").assert().success();
    env
}

/// Lines printed by `rt todo list args`, with the trailing spaces of the
/// padded names collapsed.
fn todo_list(env: &Env, args: &[&str]) -> Vec<String> {
    let assert = env
        .rt()
        .args(["todo", "list"])
        .args(args)
        .assert()
        .success();
    stdout(&assert)
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

#[test]
fn todo_list_shows_the_repositories_to_handle() {
    let env = tree();

    let mut lines = todo_list(&env, &[]);
    let summary = lines.pop();
    lines.sort();

    assert_eq!(
        lines,
        [
            "owner/conflicted unpushed commits, has conflicts",
            "owner/unpushed unpushed commits",
        ]
    );
    assert_eq!(summary.as_deref(), Some("2 todo, 1 OK, 1 N/A, 2 skipped"));
}

#[test]
fn todo_list_verbose_shows_every_repository() {
    let env = tree();

    let mut lines = todo_list(&env, &["--verbose"]);
    lines.pop();
    lines.sort();

    assert_eq!(
        lines,
        [
            "notes Ignored (local)",
            "owner/conflicted unpushed commits, has conflicts",
            "owner/git N/A",
            "owner/ignored Ignored (configuration)",
            "owner/ok OK",
            "owner/unpushed unpushed commits",
        ]
    );
}

#[test]
fn todo_list_with_filter() {
    let env = tree();

    assert_eq!(
        todo_list(&env, &["-N", "owner/ok"]),
        ["0 todo, 1 OK, 0 N/A, 0 skipped"]
    );
}

#[test]
fn todo_without_subcommand_is_a_usage_error() {
    let env = Env::new();

    env.rt().arg("todo").assert().code(2);
}

/// Environment with three repositories to handle, and their names in the
/// order `rt todo` walks them.
fn todo_tree() -> (Env, Vec<String>) {
    let env = Env::new();
    for name in ["owner/a", "owner/b", "owner/c"] {
        let repo = env.clone(name, "jujutsu-git");
        env.jj_commit(&repo, "file", "change\n");
    }
    env.clone("owner/ok", "jujutsu-git");
    env.rt().arg("refresh-cache").assert().success();
    let mut order = todo_list(&env, &[]);
    order.pop();
    let order = order
        .iter()
        .map(|line| line.split(' ').next().unwrap_or_default().to_string())
        .collect();
    (env, order)
}

/// Repository printed by `rt todo action` run from `dir`, relative to the
/// tree.
fn todo_from(env: &Env, dir: &str, action: &str) -> String {
    let assert = env
        .sh(&format!("cd '{dir}' && exec \"$RT\" todo {action}"))
        .assert()
        .success();
    stdout(&assert)
        .trim_end()
        .trim_start_matches(&format!(
            "{}/",
            env.root.join("dev/example").display()
        ))
        .to_string()
}

#[test]
fn todo_next_and_prev_from_outside_of_any_repository() {
    let (env, order) = todo_tree();
    let root = env.root.display().to_string();

    assert_eq!(todo_from(&env, &root, "next"), order[0]);
    assert_eq!(todo_from(&env, &root, "prev"), order[2]);
}

#[test]
fn todo_next_and_prev_from_a_repository() {
    let (env, order) = todo_tree();
    let dir = env.dev_path(&order[1]).display().to_string();

    assert_eq!(todo_from(&env, &dir, "next"), order[2]);
    assert_eq!(todo_from(&env, &dir, "prev"), order[0]);
}

#[test]
fn todo_next_from_a_workspace_terminates() {
    let (env, order) = todo_tree();
    let agent = env.jj_agent_workspace(&env.dev_path(&order[0]), &order[0]);
    env.rt().arg("refresh-cache").assert().success();

    let next = todo_from(&env, &agent.display().to_string(), "next");

    assert!(order.contains(&next), "{next}");
}

#[test]
fn todo_next_with_nothing_to_do() {
    let env = Env::new();
    env.clone("owner/ok", "jujutsu-git");

    let assert = env.rt().args(["todo", "next"]).assert().success();

    assert_eq!(stdout(&assert), "");
    assert!(
        stderr(&assert).contains("Nothing to do."),
        "{}",
        stderr(&assert)
    );
}
