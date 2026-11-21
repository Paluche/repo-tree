use std::process::Command;

#[test]
fn test_completion() {
    // The executable path, is present in the generated completion scripts.
    // To fix issues based on the paths to rt, being different from one computer
    // to another, especially with CI, obtain the directory containing rt, from
    // from_path. Append environment variable PATH so we can call the rt by
    // simply `rt` and not the full path.
    let rt_path = assert_cmd::cargo::cargo_bin!("rt").parent().unwrap();
    let path = format!(
        "{}:{}",
        rt_path.display(),
        std::env::var_os("PATH").unwrap().into_string().unwrap()
    );

    let mut cmd = Command::new("rt");
    cmd.env("COMPLETE", "zsh");
    cmd.env("PATH", path);
    let output = cmd.output().unwrap().stdout;

    insta::assert_snapshot!(String::from_utf8(output).unwrap());
}

mod common;

#[test]
fn todo_helper_completes_the_todo_command() {
    let env = common::Env::new();

    let assert = env.rt().env("COMPLETE", "zsh").assert().success();

    let script = common::stdout(&assert);
    assert!(script.contains("function todo()"), "{script}");
    let todo = &script[script.find("function todo()").expect("todo helper")..];
    assert!(
        todo.contains(r#"local words=("${words[0]}"  "todo" "${words[@]:1}")"#),
        "{todo}"
    );
}

#[test]
fn utils_are_only_generated_for_zsh() {
    let env = common::Env::new();

    let assert = env.rt().env("COMPLETE", "bash").assert().success();

    assert!(
        common::stderr(&assert)
            .contains("Utils generation not implemented yet for shell bash"),
        "{}",
        common::stderr(&assert)
    );
    assert_ne!(common::stdout(&assert), "");
}

#[test]
fn repository_identifiers_are_completed() {
    let env = common::Env::new();
    for name in ["owner/rocket", "owner/pocket"] {
        env.git_repo(&env.dev_path(name), Some(&common::Env::remote_url(name)));
    }

    let assert = env
        .rt()
        .env("COMPLETE", "zsh")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .args(["--", "rt", "resolve", "rock"])
        .assert()
        .success();

    let candidates = common::stdout(&assert);
    assert!(
        candidates.lines().any(|line| line.starts_with("rocket")),
        "{candidates}"
    );
    assert!(
        !candidates.lines().any(|line| line.starts_with("pocket")),
        "{candidates}"
    );
}
