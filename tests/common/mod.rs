//! Shared harness for the integration tests.
//!
//! Every test runs `rt` in an isolated environment: a temporary `HOME`,
//! configuration directory, repo-tree root, Git and Jujutsu configuration.
//! Remotes are local bare repositories reachable through
//! `ssh://example.test/<name>.git`: `GIT_SSH_COMMAND` is a fake ssh running
//! the git command locally on the bare repository, so nothing touches the
//! network while git and jj keep the remote URL as is.
#![expect(dead_code)]

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tempfile::TempDir;

/// Host of the test remotes, configured as host `example` in the rt
/// configuration.
pub const HOST: &str = "example.test";

/// Maximum duration of an `rt` invocation, so that a regression into an
/// infinite loop fails the test instead of hanging.
const TIMEOUT: Duration = Duration::from_secs(20);

/// Isolated environment in which `rt`, `git` and `jj` are run.
pub struct Env {
    /// Owner of every file of the environment, removed on drop.
    _tmp: TempDir,
    /// Value of `HOME`.
    pub home: PathBuf,
    /// Value of `XDG_CONFIG_HOME`.
    pub config_home: PathBuf,
    /// Value of `XDG_CACHE_HOME`.
    pub cache_home: PathBuf,
    /// Root of the repo-tree.
    pub root: PathBuf,
    /// Directory containing the bare remote repositories.
    pub remotes: PathBuf,
    /// Directory prepended to `PATH`, holding fake executables.
    pub bin: PathBuf,
    /// Directory for repositories created outside of the repo-tree.
    pub outside: PathBuf,
    /// Global Git configuration file.
    git_config: PathBuf,
    /// Jujutsu configuration file.
    jj_config: PathBuf,
}

impl Env {
    /// Create a new environment with the default rt configuration (see
    /// [`Env::write_config`]).
    pub fn new() -> Self {
        // A prefix not starting with a dot, so that the default ignore
        // globs (`**/.*/**`) do not apply to the environment.
        let tmp = tempfile::Builder::new()
            .prefix("rt-test-")
            .tempdir()
            .expect("cannot create the temporary directory of the test");
        let base = tmp
            .path()
            .canonicalize()
            .expect("cannot canonicalize the temporary directory");

        let env = Self {
            home: base.join("home"),
            config_home: base.join("config"),
            cache_home: base.join("cache"),
            root: base.join("tree"),
            remotes: base.join("remotes"),
            bin: base.join("bin"),
            outside: base.join("outside"),
            git_config: base.join("gitconfig"),
            jj_config: base.join("jjconfig.toml"),
            _tmp: tmp,
        };

        for dir in [
            &env.home,
            &env.config_home,
            &env.root,
            &env.remotes,
            &env.bin,
            &env.outside,
        ] {
            fs::create_dir_all(dir).expect("cannot create test directory");
        }

        write(
            &env.git_config,
            "[user]\n\tname = Test\n\temail = \
             test@example.test\n[init]\n\tdefaultBranch = main\n[protocol \
             \"file\"]\n\tallow = always\n",
        );
        write(
            &env.jj_config,
            "[user]\nname = 'Test'\nemail = \
             'test@example.test'\n[ui]\npaginate = 'never'\ncolor = \
             'never'\n[snapshot]\nauto-track = 'all()'\n",
        );
        // The last argument of ssh is the command to run on the host, on
        // the repository path given relative to the root of the remotes.
        env.fake_bin(
            "fake-ssh",
            &format!(
                "for last; do :; done\nexec sh -c \"$(printf '%s' \"$last\" | \
                 sed \"s|'/|'{}/|\")\"",
                env.remotes.display()
            ),
        );
        env.write_config("");
        env
    }

    /// Write the rt configuration: the root of the tree, the `example.test`
    /// host, no ignored location, followed by `extra`.
    pub fn write_config(&self, extra: &str) {
        self.write_raw_config(&format!(
            "root = '{}'\n\n[host.'{HOST}']\nname = \
             'example'\n\n[repository]\nignore = []\nextend_ignore = \
             []\n\n{extra}",
            self.root.display()
        ));
    }

    /// Write `content` as the rt configuration file.
    pub fn write_raw_config(&self, content: &str) {
        write(&self.config_file(), content);
    }

    /// Path of the rt configuration file.
    pub fn config_file(&self) -> PathBuf {
        self.config_home.join("repo-tree/config.toml")
    }

    /// Path of the rt cache file.
    pub fn cache_file(&self) -> PathBuf {
        self.cache_home.join("repo-tree/repo-tree.toml")
    }

    /// Apply the isolated environment variables to `cmd`.
    fn apply_env(&self, cmd: &mut Command) {
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = vec![self.bin.clone()];
        paths.extend(std::env::split_paths(&path));
        cmd.env_clear()
            .env(
                "PATH",
                std::env::join_paths(paths).expect("invalid PATH entry"),
            )
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("XDG_CACHE_HOME", &self.cache_home)
            .env("GIT_CONFIG_GLOBAL", &self.git_config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_SSH_COMMAND", self.bin.join("fake-ssh"))
            .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
            .env("JJ_CONFIG", &self.jj_config)
            .env("JJ_TIMESTAMP", "2026-01-01T00:00:00Z")
            .env("LANG", "C")
            .env("TERM", "dumb");
    }

    /// Command running `rt` in the environment, from the root of the tree,
    /// without colors.
    pub fn rt(&self) -> assert_cmd::Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("rt"));
        self.apply_env(&mut cmd);
        cmd.current_dir(&self.root);
        let mut cmd = assert_cmd::Command::from_std(cmd);
        cmd.timeout(TIMEOUT).arg("--color").arg("never");
        cmd
    }

    /// Command running the shell `script` in the environment, from the root
    /// of the tree. The path to `rt` is available in `${RT}`.
    pub fn sh(&self, script: &str) -> assert_cmd::Command {
        let mut cmd = Command::new("/bin/sh");
        self.apply_env(&mut cmd);
        cmd.current_dir(&self.root)
            .env("RT", assert_cmd::cargo::cargo_bin!("rt"))
            .arg("-c")
            .arg(script);
        let mut cmd = assert_cmd::Command::from_std(cmd);
        cmd.timeout(TIMEOUT);
        cmd
    }

    /// Run `program` with `args` in `dir`, panicking with its output if it
    /// fails. Returns its standard output.
    fn run(&self, program: &str, dir: &Path, args: &[&str]) -> String {
        let mut cmd = Command::new(program);
        self.apply_env(&mut cmd);
        let output = cmd
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("cannot run {program}: {e}"));
        assert!(
            output.status.success(),
            "{program} {args:?} in {} failed:\n{}{}",
            dir.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        String::from_utf8(output.stdout).expect("non UTF-8 output")
    }

    /// Run `git` with `args` in `dir`.
    pub fn git(&self, dir: &Path, args: &[&str]) -> String {
        self.run("git", dir, args)
    }

    /// Run `jj` with `args` in `dir`.
    pub fn jj(&self, dir: &Path, args: &[&str]) -> String {
        self.run("jj", dir, args)
    }

    /// URL of the remote `name`, as seen by rt.
    pub fn remote_url(name: &str) -> String {
        format!("ssh://{HOST}/{name}.git")
    }

    /// Path in the tree of a repository of the `dev` tree-space cloned from
    /// the remote `name`.
    pub fn dev_path(&self, name: &str) -> PathBuf {
        self.root.join("dev/example").join(name)
    }

    /// Create the bare remote `name` (e.g. `owner/repo`) with one commit on
    /// `main`. Returns its URL as seen by rt.
    pub fn bare_remote(&self, name: &str) -> String {
        let bare = self.remotes.join(format!("{name}.git"));
        fs::create_dir_all(&bare).expect("cannot create the remote");
        self.git(&bare, &["init", "--bare", "--quiet"]);

        let work = self
            .outside
            .join(format!("seed-{}", name.replace('/', "-")));
        fs::create_dir_all(&work).expect("cannot create the seed directory");
        self.git(&work, &["init", "--quiet"]);
        write(&work.join("README"), &format!("{name}\n"));
        self.git(&work, &["add", "README"]);
        self.git(&work, &["commit", "--quiet", "-m", "Initial commit"]);
        self.git(
            &work,
            &[
                "push",
                "--quiet",
                bare.to_str().expect("non UTF-8 test path"),
                "main",
            ],
        );
        fs::remove_dir_all(&work).expect("cannot remove the seed directory");
        Self::remote_url(name)
    }

    /// Initialize a Git repository at `path` with one commit and, if given,
    /// an `origin` remote with `url`.
    pub fn git_repo(&self, path: &Path, url: Option<&str>) {
        fs::create_dir_all(path).expect("cannot create the repository");
        self.git(path, &["init", "--quiet"]);
        write(&path.join("file"), "content\n");
        self.git(path, &["add", "file"]);
        self.git(path, &["commit", "--quiet", "-m", "Initial commit"]);
        if let Some(url) = url {
            self.git(path, &["remote", "add", "origin", url]);
        }
    }

    /// Initialize a colocated Jujutsu repository at `path` and, if given, an
    /// `origin` remote with `url`.
    pub fn jj_repo(&self, path: &Path, url: Option<&str>) {
        fs::create_dir_all(path).expect("cannot create the repository");
        self.jj(path, &["git", "init", "--colocate", "--quiet"]);
        if let Some(url) = url {
            self.jj(path, &["git", "remote", "add", "origin", url]);
        }
    }

    /// Clone the remote `name` with `rt clone` using `vcs`, returning the
    /// path of the clone.
    pub fn clone(&self, name: &str, vcs: &str) -> PathBuf {
        let url = self.bare_remote(name);
        self.rt()
            .args(["clone", "--vcs", vcs, &url])
            .assert()
            .success();
        self.dev_path(name)
    }

    /// Change ID of the revision `revision` of the jj repository `repo`.
    pub fn jj_change_id(&self, repo: &Path, revision: &str) -> String {
        self.jj(
            repo,
            &["log", "--no-graph", "-r", revision, "-T", "change_id"],
        )
    }

    /// Create in the jj repository `repo` a commit writing `content` in
    /// `file`, on top of the working copy.
    pub fn jj_commit(&self, repo: &Path, file: &str, content: &str) {
        write(&repo.join(file), content);
        self.jj(repo, &["commit", "-m", &format!("Write {file}")]);
    }

    /// Make the working copy of the jj repository `repo` a conflicted merge
    /// of two commits on top of `trunk()`.
    pub fn jj_conflict(&self, repo: &Path) {
        self.jj(repo, &["new", "trunk()"]);
        self.jj_commit(repo, "conflict", "a\n");
        let a = self.jj_change_id(repo, "@-");
        self.jj(repo, &["new", "trunk()"]);
        self.jj_commit(repo, "conflict", "b\n");
        let b = self.jj_change_id(repo, "@-");
        self.jj(repo, &["new", &a, &b]);
    }

    /// Add the agent workspace of the jj repository `main` of the dev
    /// tree-space named `name`, returning its path.
    pub fn jj_agent_workspace(&self, main: &Path, name: &str) -> PathBuf {
        let agent = self.root.join("agent/example").join(name);
        fs::create_dir_all(agent.parent().expect("has a parent"))
            .expect("cannot create directory");
        self.jj(
            main,
            &[
                "workspace",
                "add",
                "--name",
                "agent",
                agent.to_str().expect("UTF-8 path"),
            ],
        );
        agent
    }

    /// Install a fake executable `name` running the shell `script`.
    pub fn fake_bin(&self, name: &str, script: &str) {
        use std::os::unix::fs::PermissionsExt;

        let path = self.bin.join(name);
        write(&path, &format!("#!/bin/sh\n{script}\n"));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("cannot make the fake executable executable");
    }
}

/// Write `content` to `path`, creating the parent directories.
pub fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("cannot create parent directory");
    }
    fs::write(path, content)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}

/// Standard output of an `rt` assertion, as a string.
pub fn stdout(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone())
        .expect("non UTF-8 standard output")
}

/// Standard error of an `rt` assertion, as a string.
pub fn stderr(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stderr.clone())
        .expect("non UTF-8 standard error")
}

/// Sorted lines of `text`.
pub fn sorted_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    lines.sort();
    lines
}
