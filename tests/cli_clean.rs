//! `rt clean`: move the repositories where they belong and remove the empty
//! directories.
mod common;

use common::Env;
use common::stderr;
use common::stdout;

#[test]
fn clean_tree_has_nothing_to_do() {
    let env = Env::new();
    env.git_repo(
        &env.dev_path("owner/repo"),
        Some(&Env::remote_url("owner/repo")),
    );
    env.git_repo(&env.root.join("local/project"), None);

    let assert = env.rt().arg("clean").assert().success();

    assert_eq!(
        stdout(&assert),
        "All repositories are where they belong\nNo empty directories to \
         remove\n"
    );
}

#[test]
fn misplaced_repositories_are_moved() {
    let env = Env::new();
    let misplaced = env.dev_path("wrong/place");
    env.git_repo(&misplaced, Some(&Env::remote_url("owner/repo")));
    let local = env.dev_path("local-project");
    env.git_repo(&local, None);

    let assert = env.rt().arg("clean").assert().success();

    let out = stdout(&assert);
    assert!(out.contains("Repositories to move:"), "{out}");
    assert!(
        out.contains(&format!(
            "- owner/repo: {} => {}",
            misplaced.display(),
            env.dev_path("owner/repo").display()
        )),
        "{out}"
    );
    assert!(env.dev_path("owner/repo/.git").is_dir());
    assert!(env.root.join("local/local-project/.git").is_dir());
    // The directories left empty by the moves are removed.
    assert!(!env.dev_path("wrong").exists());
    assert!(out.contains("Removing empty directory"), "{out}");
}

#[test]
fn dry_run_changes_nothing_and_terminates() {
    let env = Env::new();
    let misplaced = env.dev_path("wrong/place");
    env.git_repo(&misplaced, Some(&Env::remote_url("owner/repo")));
    let empty = env.dev_path("empty/nested");
    std::fs::create_dir_all(&empty).expect("cannot create directory");

    let assert = env.rt().args(["clean", "--dry-run"]).assert().success();

    let out = stdout(&assert);
    assert!(out.contains("Repositories to move:"), "{out}");
    assert!(
        out.contains(&format!(
            "Empty directory to remove: {}",
            empty.display()
        )),
        "{out}"
    );
    assert!(misplaced.join(".git").is_dir());
    assert!(empty.is_dir());
}

#[test]
fn nested_empty_directories_are_removed() {
    let env = Env::new();
    env.git_repo(
        &env.dev_path("owner/repo"),
        Some(&Env::remote_url("owner/repo")),
    );
    std::fs::create_dir_all(env.dev_path("a/b/c")).expect("cannot create");

    env.rt().arg("clean").assert().success();

    assert!(!env.dev_path("a").exists());
    assert!(env.dev_path("owner/repo/.git").is_dir());
    // Git keeps empty directories in its own directory, they are not touched.
    assert!(env.dev_path("owner/repo/.git/refs/tags").is_dir());
}

#[test]
fn agent_workspaces_are_where_they_belong() {
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

    let assert = env.rt().arg("clean").assert().success();

    assert!(
        stdout(&assert).starts_with("All repositories are where they belong"),
        "{}",
        stdout(&assert)
    );
    assert!(agent.join(".jj").is_dir());
}

#[test]
fn failing_move_is_reported() {
    let env = Env::new();
    let misplaced = env.dev_path("wrong/place");
    env.git_repo(&misplaced, Some(&Env::remote_url("owner/repo")));
    // Something already lives at the expected location.
    common::write(&env.dev_path("owner/repo/file"), "");

    let assert = env.rt().arg("clean").assert().code(1);

    assert!(
        stderr(&assert).contains(&format!(
            "Cannot move {} to {}",
            misplaced.display(),
            env.dev_path("owner/repo").display()
        )),
        "{}",
        stderr(&assert)
    );
    assert!(misplaced.join(".git").is_dir());
}

#[test]
fn failing_removal_is_reported_and_terminates() {
    use std::os::unix::fs::PermissionsExt;

    let env = Env::new();
    let parent = env.dev_path("locked");
    std::fs::create_dir_all(parent.join("empty")).expect("cannot create");
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o555))
        .expect("cannot change permissions");

    let assert = env.rt().arg("clean").assert();

    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755))
        .expect("cannot change permissions");
    let assert = assert.code(1);
    assert!(
        stderr(&assert).contains("Cannot remove"),
        "{}",
        stderr(&assert)
    );
}
