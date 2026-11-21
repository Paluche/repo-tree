//! `rt repo`: information about the repository of a directory.
mod common;

use std::path::Path;

use common::Env;
use common::stderr;
use common::stdout;

/// Run `rt args` from the directory `dir`.
fn rt_in(env: &Env, dir: &Path, args: &str) -> assert_cmd::assert::Assert {
    env.sh(&format!(
        "cd '{}' && exec \"$RT\" --color never {args}",
        dir.display()
    ))
    .assert()
}

/// Remove the ANSI escape sequences of `text`.
fn strip_ansi(text: &str) -> String {
    regex::Regex::new("\x1b\\[[0-9;]*m")
        .expect("valid regular expression")
        .replace_all(text, "")
        .into_owned()
}

#[test]
fn repo_root_of_a_directory_within_the_repository() {
    let env = Env::new();
    let repo = env.root.join("local/project");
    env.git_repo(&repo, None);
    std::fs::create_dir_all(repo.join("a/b")).expect("cannot create directory");

    let assert = rt_in(&env, &repo.join("a/b"), "repo root").success();

    assert_eq!(stdout(&assert), format!("{}\n", repo.display()));
}

#[test]
fn repo_root_with_types() {
    let env = Env::new();
    let git = env.root.join("local/git");
    env.git_repo(&git, None);
    let jj = env.root.join("local/jj");
    env.jj_repo(&jj, None);

    let assert = rt_in(&env, &git, "repo root --print-type").success();
    assert_eq!(stdout(&assert), format!("{}\ntrue\nfalse\n", git.display()));

    let assert = rt_in(&env, &jj, "repo root --print-type").success();
    assert_eq!(stdout(&assert), format!("{}\ntrue\ntrue\n", jj.display()));
}

#[test]
fn repo_root_of_the_parent_of_a_submodule() {
    let env = Env::new();
    let sub_url = env.bare_remote("owner/sub");
    let main = env.root.join("local/main");
    env.git_repo(&main, None);
    env.git(&main, &["submodule", "add", "--quiet", &sub_url, "sub"]);

    let assert = rt_in(&env, &main.join("sub"), "repo root").success();
    assert_eq!(stdout(&assert), format!("{}\n", main.join("sub").display()));

    let assert = rt_in(&env, &main.join("sub"), "repo root --parent").success();
    assert_eq!(stdout(&assert), format!("{}\n", main.display()));
}

#[test]
fn repo_root_outside_of_any_repository_fails() {
    let env = Env::new();

    let assert = rt_in(&env, &env.outside, "repo root").code(1);

    assert_eq!(stdout(&assert), "");
}

#[test]
fn repo_remote_with_one_remote() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "git");

    let assert = rt_in(&env, &repo, "repo remote").success();

    assert_eq!(
        stdout(&assert),
        format!("{}\n", Env::remote_url("owner/repo"))
    );
}

#[test]
fn repo_remote_with_several_remotes_asks() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "git");
    env.git(
        &repo,
        &["remote", "add", "upstream", "ssh://example.test/up/repo"],
    );

    for (input, expected) in [
        ("1\n", "ssh://example.test/up/repo"),
        ("0\n", &Env::remote_url("owner/repo")),
        ("9\n", &Env::remote_url("owner/repo")),
        ("", &Env::remote_url("owner/repo")),
    ] {
        let assert = env
            .rt()
            .args(["repo", "remote", "-r", repo.to_str().expect("UTF-8")])
            .write_stdin(input)
            .assert()
            .success();

        assert_eq!(stdout(&assert), format!("{expected}\n"), "{input:?}");
        assert!(
            stderr(&assert).contains("origin")
                && stderr(&assert).contains("(default)"),
            "{}",
            stderr(&assert)
        );
    }
}

#[test]
fn repo_remote_without_remote_fails() {
    let env = Env::new();
    let repo = env.root.join("local/project");
    env.git_repo(&repo, None);

    let assert = rt_in(&env, &repo, "repo remote").code(1);

    assert_eq!(stdout(&assert), "");
    assert!(
        stderr(&assert).contains("No remote URL found"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn repo_remote_outside_of_any_repository_fails() {
    let env = Env::new();

    let assert = rt_in(&env, &env.outside, "repo remote").code(1);

    assert_eq!(stdout(&assert), "");
    assert!(
        stderr(&assert).contains("No repository found"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn repo_state_of_jj_repositories() {
    let env = Env::new();
    let ok = env.clone("owner/ok", "jujutsu-git");
    let unpushed = env.clone("owner/unpushed", "jujutsu-git");
    env.jj_commit(&unpushed, "file", "change\n");
    let conflicted = env.clone("owner/conflicted", "jujutsu-git");
    env.jj_conflict(&conflicted);
    // A commit on top of a main which moved on the remote.
    let restack = env.clone("owner/restack", "jujutsu-git");
    env.jj_commit(&restack, "file", "change\n");
    let work = env.outside.join("work");
    env.git(
        &env.outside,
        &[
            "clone",
            "--quiet",
            &Env::remote_url("owner/restack"),
            "work",
        ],
    );
    common::write(&work.join("other"), "other\n");
    env.git(&work, &["add", "other"]);
    env.git(&work, &["commit", "--quiet", "-m", "Other"]);
    env.git(&work, &["push", "--quiet", "origin", "main"]);
    env.jj(&restack, &["git", "fetch"]);

    for (repo, expected) in [
        (&ok, "OK"),
        (&unpushed, "unpushed commits"),
        (&conflicted, "unpushed commits, has conflicts"),
        (&restack, "unpushed commits, needs restack"),
    ] {
        let assert = rt_in(&env, repo, "repo state").success();
        assert_eq!(
            stdout(&assert),
            format!("{expected}\n"),
            "{}",
            repo.display()
        );
    }
}

#[test]
fn repo_state_verbose() {
    let env = Env::new();
    let repo = env.clone("owner/ok", "jujutsu-git");

    let assert = rt_in(&env, &repo, "repo state --verbose").success();

    let lines: Vec<String> =
        stdout(&assert).lines().map(str::to_string).collect();
    assert_eq!(
        lines[0].trim_end(),
        format!("example  owner/ok: {}", Env::remote_url("owner/ok"))
    );
    assert!(lines[1].starts_with("jj-git "), "{lines:?}");
    assert_eq!(lines[2], "OK");
}

#[test]
fn repo_state_of_git_repository_is_not_implemented() {
    let env = Env::new();
    let repo = env.clone("owner/git", "git");

    let assert = rt_in(&env, &repo, "repo state").code(1);

    assert!(
        stderr(&assert).contains("not implemented"),
        "{}",
        stderr(&assert)
    );
}

#[test]
fn repo_state_outside_of_any_repository_fails() {
    let env = Env::new();

    rt_in(&env, &env.outside, "repo state").code(1);
}

#[test]
fn repo_prompt_outside_of_any_repository_is_empty() {
    let env = Env::new();

    let assert = rt_in(&env, &env.outside, "repo prompt").success();

    assert_eq!(stdout(&assert), "");
}

/// Prompt of the repository `repo`, without colors.
fn prompt(env: &Env, repo: &Path) -> String {
    let assert = rt_in(env, repo, "repo prompt").success();
    strip_ansi(stdout(&assert).trim_end())
}

#[test]
fn repo_prompt_of_git_repository() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "git");
    let clean = prompt(&env, &repo);

    common::write(&repo.join("README"), "modified\n");
    common::write(&repo.join("staged"), "staged\n");
    env.git(&repo, &["add", "staged"]);
    common::write(&repo.join("untracked"), "untracked\n");
    let changed = prompt(&env, &repo);

    env.git(&repo, &["stash", "--quiet"]);
    env.git(
        &repo,
        &["commit", "--quiet", "--allow-empty", "-m", "Ahead"],
    );
    let ahead = prompt(&env, &repo);

    env.git(&repo, &["checkout", "--quiet", "--detach"]);
    let detached = prompt(&env, &repo);

    insta::assert_snapshot!([clean, changed, ahead, detached].join("\n"));
}

#[test]
fn repo_prompt_of_jj_repository() {
    let env = Env::new();
    let repo = env.clone("owner/repo", "jujutsu-git");
    let clean = prompt(&env, &repo);

    env.jj_commit(&repo, "file", "change\n");
    env.jj(&repo, &["bookmark", "create", "feature", "-r", "@-"]);
    env.jj(&repo, &["tag", "set", "v1", "-r", "main"]);
    let bookmarks = prompt(&env, &repo);

    env.jj(&repo, &["bookmark", "delete", "main"]);
    let deleted = prompt(&env, &repo);

    env.jj_conflict(&repo);
    let conflict = prompt(&env, &repo);

    let agent = env.jj_agent_workspace(&repo, "owner/repo");
    let workspace = prompt(&env, &agent);

    insta::assert_snapshot!(
        [clean, bookmarks, deleted, conflict, workspace].join("\n")
    );
}
