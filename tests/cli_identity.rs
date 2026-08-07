//! Management of the identities used in the repositories.
mod common;

use std::path::PathBuf;

use common::Env;
use common::stderr;
use common::stdout;

/// Identities of the tests: a default one, and a `work` one overriding the
/// email of the repositories of `company/`.
const IDENTITIES: &str = "
[identities.default]
name = 'Default Name'
email = 'default@example.test'

[identities.work]
email = 'work@company.test'
scopes = [{ host = 'example.test', path = 'company/' }]
";

/// Create an environment with the test identities and a Git repository
/// `name` of the `dev` tree-space, returning the path of the repository.
fn env_with_repo(name: &str) -> (Env, PathBuf) {
    let env = Env::new();
    env.write_config(IDENTITIES);
    let repo = env.dev_path(name);
    env.git_repo(&repo, Some(&Env::remote_url(name)));
    (env, repo)
}

#[test]
fn show_merges_the_identities_of_the_scopes() {
    let (env, repo) = env_with_repo("company/project");

    let assert = env
        .rt()
        .current_dir(&repo)
        .args(["identity", "show"])
        .assert()
        .success();

    let stdout = stdout(&assert);
    assert!(stdout.contains("identities: default, work\n"), "{stdout}");
    assert!(stdout.contains("name:       Default Name\n"), "{stdout}");
    assert!(
        stdout.contains("email:      work@company.test\n"),
        "{stdout}"
    );
    assert!(stdout.contains("signing:    not managed\n"), "{stdout}");
}

#[test]
fn show_uses_the_default_identity_out_of_the_scopes() {
    let (env, repo) = env_with_repo("other/project");

    let assert = env
        .rt()
        .args(["identity", "show", "--repository"])
        .arg(&repo)
        .assert()
        .success();

    let stdout = stdout(&assert);
    assert!(stdout.contains("identities: default\n"), "{stdout}");
    assert!(
        stdout.contains("email:      default@example.test\n"),
        "{stdout}"
    );
}

#[test]
fn show_without_identity_fails() {
    let env = Env::new();
    let repo = env.dev_path("company/project");
    env.git_repo(&repo, Some(&Env::remote_url("company/project")));

    let assert = env
        .rt()
        .current_dir(&repo)
        .args(["identity", "show"])
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("No identity configured"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn show_lists_the_configuration_to_change() {
    let (env, repo) = env_with_repo("company/project");

    let assert = env
        .rt()
        .current_dir(&repo)
        .args(["identity", "show"])
        .assert()
        .success();

    let stdout = stdout(&assert);
    assert!(
        stdout.contains(
            "The repository configuration differs, see rt identity apply:\n  \
             git user.name: unset -> \"Default Name\"\n  git user.email: \
             unset -> \"work@company.test\"\n"
        ),
        "{stdout}"
    );
}

#[test]
fn apply_sets_the_identity_of_a_git_repository() {
    let (env, repo) = env_with_repo("company/project");

    let assert = env
        .rt()
        .current_dir(&repo)
        .args(["identity", "apply"])
        .assert()
        .success();

    assert_eq!(
        stdout(&assert),
        "example company/project ssh://example.test/company/project.git\n  \
         git user.name: unset -> \"Default Name\"\n  git user.email: unset -> \
         \"work@company.test\"\n"
    );
    assert_eq!(
        env.git(&repo, &["config", "--local", "user.email"]),
        "work@company.test\n"
    );

    // Nothing left to change.
    let assert = env
        .rt()
        .current_dir(&repo)
        .args(["identity", "apply"])
        .assert()
        .success();
    assert_eq!(stdout(&assert), "");
    let assert = env
        .rt()
        .current_dir(&repo)
        .args(["identity", "show"])
        .assert()
        .success();
    assert!(!stdout(&assert).contains("differs"), "{}", stdout(&assert));
}

#[test]
fn apply_sets_the_identity_of_a_jujutsu_repository() {
    let env = Env::new();
    env.write_config(IDENTITIES);
    let repo = env.dev_path("company/project");
    env.jj_repo(&repo, Some(&Env::remote_url("company/project")));

    env.rt()
        .current_dir(&repo)
        .args(["identity", "apply"])
        .assert()
        .success();

    assert_eq!(
        env.jj(&repo, &["config", "get", "user.email"]),
        "work@company.test\n"
    );
    // Colocated: Git gets the identity too.
    assert_eq!(
        env.git(&repo, &["config", "--local", "user.email"]),
        "work@company.test\n"
    );
}

#[test]
fn apply_all_sets_the_identity_of_every_repository() {
    let (env, work) = env_with_repo("company/project");
    let personal = env.dev_path("other/project");
    env.git_repo(&personal, Some(&Env::remote_url("other/project")));

    env.rt()
        .args(["identity", "apply", "--all"])
        .assert()
        .success();

    assert_eq!(
        env.git(&work, &["config", "--local", "user.email"]),
        "work@company.test\n"
    );
    assert_eq!(
        env.git(&personal, &["config", "--local", "user.email"]),
        "default@example.test\n"
    );
}

#[test]
fn apply_without_identity_fails() {
    let env = Env::new();

    let assert = env
        .rt()
        .args(["identity", "apply", "--all"])
        .assert()
        .code(1);

    assert!(
        stderr(&assert).contains("No identity configured"),
        "{}",
        stderr(&assert)
    );
}
