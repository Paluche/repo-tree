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
