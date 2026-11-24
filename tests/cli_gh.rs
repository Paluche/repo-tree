//! `rt gh`: the GitHub CLI with the token of the identity.
mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use common::Env;
use common::stderr;
use common::stdout;

/// Identities of the tests: the `work` identity has its own API token for
/// the repositories of `company/` on github.com.
const IDENTITIES: &str = "
[identities.default]
name = 'Default Name'
email = 'default@example.test'
tokens.'github.com' = { api = 'personal' }
tokens.'example.test' = { api = 'enterprise' }

[identities.work]
tokens.'github.com' = { api = 'work' }
scopes = [{ host = 'github.com', path = 'company/' }]
";

/// Secrets of the tests.
const SECRETS: &str = "
[personal]
token = 'personal-token'

[work]
token = 'work-token'

[enterprise]
token = 'enterprise-token'
";

/// Create an environment with a fake `gh` printing its authentication
/// environment variables and its arguments, and the test identities and
/// secrets if `identities`.
fn env(identities: bool) -> Env {
    let env = Env::new();
    env.fake_bin(
        "gh",
        "printf 'GH_TOKEN=%s\\nGH_HOST=%s\\nGH_ENTERPRISE_TOKEN=%s\\n' \
         \"$GH_TOKEN\" \"$GH_HOST\" \"$GH_ENTERPRISE_TOKEN\"\nprintf '%s\\n' \
         \"$@\"",
    );
    if identities {
        env.write_config(IDENTITIES);
        let secrets = env.config_home.join("repo-tree/secrets.toml");
        common::write(&secrets, SECRETS);
        fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600))
            .expect("cannot set the permissions of the secrets");
    }
    env
}

/// Expected output of the fake `gh`.
fn gh_output(
    token: &str,
    host: &str,
    enterprise_token: &str,
    args: &[&str],
) -> String {
    let mut lines = vec![
        format!("GH_TOKEN={token}"),
        format!("GH_HOST={host}"),
        format!("GH_ENTERPRISE_TOKEN={enterprise_token}"),
    ];
    lines.extend(args.iter().map(ToString::to_string));
    let mut ret = lines.join("\n");
    ret.push('\n');
    ret
}

#[test]
fn repo_option_selects_the_identity() {
    let env = env(true);

    let assert = env
        .rt()
        .args(["gh", "pr", "list", "-R", "company/project"])
        .assert()
        .success();
    assert_eq!(
        stdout(&assert),
        gh_output(
            "work-token",
            "",
            "",
            &["pr", "list", "-R", "company/project"]
        )
    );

    let assert = env
        .rt()
        .args(["gh", "pr", "list", "--repo=other/project"])
        .assert()
        .success();
    assert_eq!(
        stdout(&assert),
        gh_output(
            "personal-token",
            "",
            "",
            &["pr", "list", "--repo=other/project"]
        )
    );
}

#[test]
fn current_repository_selects_the_identity() {
    let env = env(true);
    let repo = env.dev_path("project");
    env.git_repo(&repo, Some(&Env::remote_url("project")));

    let assert = env
        .rt()
        .current_dir(&repo)
        .args(["gh", "issue", "list"])
        .assert()
        .success();

    assert_eq!(
        stdout(&assert),
        gh_output("", "example.test", "enterprise-token", &["issue", "list"])
    );
}

#[test]
fn outside_repositories_the_default_identity_is_used() {
    let env = env(true);

    let assert = env.rt().args(["gh", "api", "user"]).assert().success();

    assert_eq!(
        stdout(&assert),
        gh_output("personal-token", "", "", &["api", "user"])
    );
}

#[test]
fn without_identity_gh_runs_as_it_is() {
    let env = env(false);

    let assert = env.rt().args(["gh", "--version"]).assert().success();

    assert_eq!(stdout(&assert), gh_output("", "", "", &["--version"]));
}

#[test]
fn missing_api_token_is_reported() {
    let env = env(true);
    env.write_config(
        "[identities.default]\nname = 'Default Name'\nemail = \
         'default@example.test'\n",
    );

    let assert = env.rt().args(["gh", "api", "user"]).assert().code(1);

    assert!(
        stderr(&assert).contains("has no API token for github.com"),
        "{}",
        stderr(&assert)
    );
}
