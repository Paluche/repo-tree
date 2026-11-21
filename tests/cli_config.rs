//! Loading of the configuration, as seen from the command line.
mod common;

use common::Env;
use common::stderr;
use common::stdout;

#[test]
fn missing_config_file_uses_repo_tree_dir() {
    let env = Env::new();
    std::fs::remove_file(env.config_file()).expect("cannot remove the config");
    let repo = env.root.join("local/project");
    env.git_repo(&repo, None);

    let assert = env
        .rt()
        .env("REPO_TREE_DIR", &env.root)
        .arg("list")
        .assert()
        .success();

    assert_eq!(stdout(&assert), format!("{}\n", repo.display()));
}

#[test]
fn relative_root_is_rejected() {
    let env = Env::new();
    env.write_raw_config("root = 'relative/path'\n");

    let assert = env.rt().arg("list").assert().code(1);

    assert!(
        stderr(&assert).contains("Bad configuration"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn invalid_toml_is_reported() {
    let env = Env::new();
    env.write_raw_config("root = \n");

    let assert = env.rt().arg("list").assert().code(1);

    assert!(
        stderr(&assert).contains("TOML parse error"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn wrong_value_type_is_reported() {
    let env = Env::new();
    env.write_config("[command.clone]\ndefault_vcs = 'svn'\n");

    let assert = env.rt().arg("list").assert().code(1);

    assert!(
        stderr(&assert).contains("default_vcs"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn deleted_current_directory_is_reported() {
    let env = Env::new();
    std::fs::create_dir(env.outside.join("gone"))
        .expect("cannot create directory");

    let assert = env
        .sh("cd ../outside/gone && rmdir ../gone && exec \"$RT\" git status")
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("Current directory does not exist"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn missing_root_is_reported() {
    let env = Env::new();
    env.write_raw_config("");

    let assert = env.rt().arg("list").assert().code(1);

    assert!(
        stderr(&assert).contains("REPO_TREE_DIR"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn partial_repository_table_is_accepted() {
    let env = Env::new();
    env.write_raw_config(&format!(
        "root = '{}'\n[repository]\nignore = []\n",
        env.root.display()
    ));

    env.rt().arg("list").assert().success();
}
