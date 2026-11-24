//! `rt credential`: the Git credential helper.
mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use common::Env;
use common::stderr;
use common::stdout;

/// Identities of the tests: the `work` identity has its own Git token for
/// the repositories of `company/`.
const IDENTITIES: &str = "
[identities.default]
name = 'Default Name'
email = 'default@example.test'
tokens.'example.test' = { git = 'personal' }

[identities.work]
tokens.'example.test' = { git = 'work' }
scopes = [{ host = 'example.test', path = 'company/' }]
";

/// Secrets of the tests.
const SECRETS: &str = "
[personal]
username = 'me'
token = 'personal-token'

[work]
username = 'me-at-work'
token = 'work-token'
";

/// Create an environment with the test identities and secrets, the secrets
/// file having the permissions `mode`.
fn env_with_secrets(mode: u32) -> Env {
    let env = Env::new();
    env.write_config(IDENTITIES);
    let secrets = env.config_home.join("repo-tree/secrets.toml");
    common::write(&secrets, SECRETS);
    fs::set_permissions(&secrets, fs::Permissions::from_mode(mode))
        .expect("cannot set the permissions of the secrets");
    env
}

/// Run `rt credential get` with the credential `request`.
fn get(env: &Env, request: &str) -> assert_cmd::assert::Assert {
    env.rt()
        .args(["credential", "get"])
        .write_stdin(request)
        .assert()
        .success()
}

#[test]
fn get_gives_the_token_of_the_identity_of_the_path() {
    let env = env_with_secrets(0o600);

    let assert = get(
        &env,
        "protocol=https\nhost=example.test\npath=company/project.git\n\n",
    );
    assert_eq!(
        stdout(&assert),
        "username=me-at-work\npassword=work-token\n"
    );
    assert_eq!(stderr(&assert), "");

    let assert = get(
        &env,
        "protocol=https\nhost=example.test\npath=other/project.git\n\n",
    );
    assert_eq!(stdout(&assert), "username=me\npassword=personal-token\n");
}

#[test]
fn get_without_path_uses_the_current_repository() {
    let env = env_with_secrets(0o600);
    let repo = env.dev_path("company/project");
    env.git_repo(&repo, Some(&Env::remote_url("company/project")));

    let assert = env
        .rt()
        .current_dir(&repo)
        .args(["credential", "get"])
        .write_stdin("protocol=https\nhost=example.test\n\n")
        .assert()
        .success();

    assert_eq!(
        stdout(&assert),
        "username=me-at-work\npassword=work-token\n"
    );
}

#[test]
fn get_ignores_other_protocols_and_hosts() {
    let env = env_with_secrets(0o600);

    for request in [
        "protocol=http\nhost=example.test\npath=company/project.git\n\n",
        "protocol=https\nhost=elsewhere.test\npath=company/project.git\n\n",
    ] {
        let assert = get(&env, request);
        assert_eq!(stdout(&assert), "", "{request}");
    }
}

#[test]
fn store_and_erase_do_nothing() {
    let env = env_with_secrets(0o600);
    let request =
        "protocol=https\nhost=example.test\nusername=me\npassword=x\n\n";

    for operation in ["store", "erase"] {
        let assert = env
            .rt()
            .args(["credential", operation])
            .write_stdin(request)
            .assert()
            .success();
        assert_eq!(stdout(&assert), "", "{operation}");
    }
}

#[test]
fn readable_secrets_are_reported() {
    let env = env_with_secrets(0o644);

    let assert = get(
        &env,
        "protocol=https\nhost=example.test\npath=company/project.git\n\n",
    );

    assert!(
        stderr(&assert).contains("can be accessed by other users"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn git_gets_the_token_from_the_helper() {
    let env = env_with_secrets(0o600);
    let repo = env.dev_path("company/project");
    env.git_repo(&repo, None);

    let output = env
        .sh("git -c credential.helper='!\"$RT\" credential' -c \
             credential.useHttpPath=true credential fill")
        .current_dir(&repo)
        .write_stdin("url=https://example.test/company/project.git\n\n")
        .assert()
        .success();

    assert!(
        stdout(&output).contains("username=me-at-work\npassword=work-token\n"),
        "{}",
        stdout(&output)
    );
}
