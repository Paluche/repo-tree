//! `rt clone` and `rt insert`: put repositories in the repo-tree.
mod common;

use common::Env;
use common::stderr;
use common::stdout;

/// Last line of the standard output, the location of the clone.
fn last_line(text: &str) -> &str {
    text.lines().last().unwrap_or_default()
}

#[test]
fn clone_with_each_version_control_system() {
    let env = Env::new();

    for (vcs, markers) in [
        ("git", &[".git"][..]),
        ("jujutsu", &[".jj"]),
        ("jujutsu-git", &[".jj", ".git"]),
    ] {
        let name = format!("owner/{vcs}");
        let url = env.bare_remote(&name);

        let assert = env
            .rt()
            .args(["clone", "--vcs", vcs, &url])
            .assert()
            .success();

        let location = env.dev_path(&name);
        assert_eq!(last_line(&stdout(&assert)), location.display().to_string());
        for marker in markers {
            assert!(location.join(marker).is_dir(), "{vcs}: {marker}");
        }
        assert!(location.join("README").is_file(), "{vcs}: checkout");
    }

    let assert = env.rt().arg("list").assert().success();
    assert_eq!(common::sorted_lines(&stdout(&assert)).len(), 3);
}

#[test]
fn clone_uses_the_configured_default_vcs() {
    let env = Env::new();
    env.write_config("[command.clone]\ndefault_vcs = 'git'\n");
    let url = env.bare_remote("owner/repo");

    env.rt().args(["clone", &url]).assert().success();

    assert!(env.dev_path("owner/repo/.git").is_dir());
    assert!(!env.dev_path("owner/repo/.jj").exists());
}

#[test]
fn clone_in_archive() {
    let env = Env::new();
    let url = env.bare_remote("owner/old");

    let assert = env
        .rt()
        .args(["clone", "--vcs", "git", "--force-tree", "archive", &url])
        .assert()
        .success();

    let location = env.root.join("archive/example/owner/old");
    assert_eq!(last_line(&stdout(&assert)), location.display().to_string());
    assert!(location.join(".git").is_dir());
}

#[test]
fn clone_initializes_git_submodules() {
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

    assert!(env.dev_path("owner/main/sub/README").is_file());
}

#[test]
fn clone_already_cloned_repository() {
    let env = Env::new();
    let location = env.clone("owner/repo", "git");

    let assert = env
        .rt()
        .args(["clone", "--vcs", "git", &Env::remote_url("owner/repo")])
        .assert()
        .success();

    assert_eq!(last_line(&stdout(&assert)), location.display().to_string());
    assert!(
        stderr(&assert).contains("repository already cloned"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn clone_jujutsu_git_over_git_clone_initializes_jj() {
    let env = Env::new();
    let location = env.clone("owner/repo", "git");

    env.rt()
        .args([
            "clone",
            "--vcs",
            "jujutsu-git",
            &Env::remote_url("owner/repo"),
        ])
        .assert()
        .success();

    assert!(location.join(".jj").is_dir());
    assert!(location.join(".git").is_dir());
}

#[test]
fn clone_over_a_directory_fails() {
    let env = Env::new();
    let url = env.bare_remote("owner/repo");
    std::fs::create_dir_all(env.dev_path("owner/repo"))
        .expect("cannot create directory");

    let assert = env.rt().args(["clone", &url]).assert().code(1);

    assert_eq!(stdout(&assert), "");
    assert!(
        stderr(&assert).contains("already exists and is not a repository"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn clone_invalid_url_fails() {
    let env = Env::new();

    let assert = env.rt().args(["clone", "not a url"]).assert().code(1);

    assert!(
        stderr(&assert).contains("Error parsing not a url"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn clone_from_unknown_host_fails() {
    let env = Env::new();

    let assert = env
        .rt()
        .args(["clone", "ssh://unknown.test/owner/repo.git"])
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("Missing host configuration for unknown.test"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn clone_of_missing_remote_fails() {
    let env = Env::new();

    for vcs in ["git", "jujutsu-git"] {
        let assert = env
            .rt()
            .args(["clone", "--vcs", vcs, &Env::remote_url("owner/missing")])
            .assert()
            .code(1);

        assert!(stderr(&assert).contains("failed"), "{}", stderr(&assert));
        assert!(!env.dev_path("owner/missing/.git").exists(), "{vcs}");
    }
}

#[test]
fn insert_moves_a_repository_into_the_tree() {
    let env = Env::new();
    let url = env.bare_remote("owner/repo");
    let parent = env.outside.join("projects/nested");
    std::fs::create_dir_all(&parent).expect("cannot create directory");
    env.git(&parent, &["clone", "--quiet", &url, "repo"]);

    let assert = env
        .rt()
        .args(["insert", parent.join("repo").to_str().expect("UTF-8")])
        .assert()
        .success();

    assert_eq!(
        stdout(&assert),
        format!(
            "{} moved to {}\n",
            parent.join("repo").display(),
            env.dev_path("owner/repo").display()
        )
    );
    assert!(env.dev_path("owner/repo/.git").is_dir());
    // Directories outside of the tree are left untouched.
    assert!(parent.is_dir());
}

#[test]
fn insert_relative_path_of_local_repository() {
    let env = Env::new();
    env.git_repo(&env.outside.join("project"), None);

    env.sh("cd ../outside && exec \"$RT\" insert project")
        .assert()
        .success();

    assert!(env.root.join("local/project/.git").is_dir());
}

#[test]
fn insert_current_directory() {
    let env = Env::new();
    env.git_repo(&env.outside.join("project"), None);

    env.sh("cd ../outside/project && exec \"$RT\" insert .")
        .assert()
        .success();

    assert!(env.root.join("local/project/.git").is_dir());
}

#[test]
fn insert_repository_already_in_place() {
    let env = Env::new();
    let location = env.clone("owner/repo", "git");

    let assert = env
        .rt()
        .args(["insert", location.to_str().expect("UTF-8")])
        .assert()
        .success();

    assert_eq!(stdout(&assert), "");
    assert!(
        stderr(&assert).contains("already at the correct location"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn insert_misplaced_tree_repository_cleans_its_old_parents() {
    let env = Env::new();
    let misplaced = env.dev_path("wrong/place/repo");
    env.git_repo(&misplaced, Some(&Env::remote_url("owner/repo")));

    env.rt()
        .args(["insert", misplaced.to_str().expect("UTF-8")])
        .assert()
        .success();

    assert!(env.dev_path("owner/repo/.git").is_dir());
    assert!(!env.dev_path("wrong").exists());
    assert!(env.root.join("dev").is_dir());
}

#[test]
fn insert_not_a_repository_fails() {
    let env = Env::new();
    let dir = env.outside.join("dir");
    std::fs::create_dir_all(&dir).expect("cannot create directory");

    let assert = env
        .rt()
        .args(["insert", dir.to_str().expect("UTF-8")])
        .assert()
        .code(1);
    assert!(
        stderr(&assert).contains("No repository found"),
        "{}",
        stderr(&assert)
    );

    let assert = env
        .rt()
        .args(["insert", "/does/not/exist"])
        .assert()
        .code(1);
    assert!(
        stderr(&assert).contains("Invalid path"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn insert_submodule_fails() {
    let env = Env::new();
    let sub_url = env.bare_remote("owner/sub");
    let main = env.outside.join("main");
    env.git_repo(&main, None);
    env.git(&main, &["submodule", "add", "--quiet", &sub_url, "sub"]);

    let assert = env
        .rt()
        .args(["insert", main.join("sub").to_str().expect("UTF-8")])
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("Repository is a submodule"),
        "{}",
        stderr(&assert)
    );
    assert!(main.join("sub/README").is_file());
}
