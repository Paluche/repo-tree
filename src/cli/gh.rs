//! Run the GitHub CLI with the API token of the identity of the repository,
//! instead of the accounts `gh` manages itself.
use std::error::Error;
use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::process::Command;

use clap::Args;
use which::which;

use crate::cli::get_cwd;
use crate::config::Config;
use crate::repo_id::Remote;
use crate::repo_id::RepoId;
use crate::repository::Repository;
use crate::secrets::Secrets;
use crate::ui::Ui;

/// Default GitHub host.
const GITHUB_HOST: &str = "github.com";

/// Run `gh` with the API token of the identity of the repository it works
/// on: the one given with `-R/--repo`, or the repository of the current
/// directory, or the default identity for github.com. Without any identity
/// configured, `gh` runs as it is.
///
/// The arguments are not parsed by rt, so global rt options cannot be given.
#[derive(Args)]
#[command(disable_help_flag = true)]
pub struct GhArgs {
    /// Arguments given to `gh` as they are.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

/// Get the value of the `-R/--repo` option of the `gh` arguments, if any.
fn repo_option(args: &[OsString]) -> Option<String> {
    let mut args = args.iter().filter_map(|a| a.to_str());

    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if arg == "-R" || arg == "--repo" {
            return args.next().map(ToString::to_string);
        }
        if let Some(value) = arg.strip_prefix("--repo=") {
            return Some(value.to_string());
        }
        if let Some(value) = arg.strip_prefix("-R")
            && !value.starts_with('-')
        {
            return Some(value.to_string());
        }
    }

    None
}

/// Build a repository identifier for a host, and a repository name, empty
/// for the host only.
fn repo_id(host: &str, name: &str) -> RepoId {
    RepoId {
        remote: Some(Remote {
            url: format!("https://{host}/{name}"),
            host_url: host.to_string(),
        }),
        name: name.to_string(),
    }
}

/// Get the repository identifier of a `-R/--repo` value: `OWNER/REPO`,
/// `HOST/OWNER/REPO` or a URL.
fn repo_id_from_option(value: &str, default_host: &str) -> Option<RepoId> {
    if value.contains("://") || value.contains('@') {
        return RepoId::from_remote_url(value).ok();
    }

    let parts: Vec<&str> = value.split('/').collect();

    match parts.as_slice() {
        [owner, repo] => {
            Some(repo_id(default_host, &format!("{owner}/{repo}")))
        }
        [host, owner, repo] => Some(repo_id(host, &format!("{owner}/{repo}"))),
        _ => None,
    }
}

/// Find the repository `gh` works on.
fn target(ui: &Ui<'_>, config: &Config, args: &[OsString]) -> RepoId {
    let default_host =
        std::env::var("GH_HOST").unwrap_or_else(|_| GITHUB_HOST.to_string());

    if let Some(value) = repo_option(args)
        && let Some(repo_id) = repo_id_from_option(&value, &default_host)
    {
        return repo_id;
    }

    if let Ok(repository) = Repository::discover_silent(config, &get_cwd(ui))
        && repository.id.remote.is_some()
    {
        return repository.id;
    }

    repo_id(&default_host, "")
}

/// Replace the process with `gh`, authenticated with the API token of the
/// identity of the repository. Without any identity configured, `gh` runs
/// as it is. Only returns on error.
fn exec_gh(
    ui: &Ui<'_>,
    config: &Config,
    args: &[OsString],
) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new(which("gh")?);
    command.args(args);

    if config.identities.is_empty() {
        return Err(Box::new(command.exec()));
    }

    let repo_id = target(ui, config, args);
    let host = &repo_id
        .remote
        .as_ref()
        .expect("target() always gives a repository with a remote")
        .host_url;
    let identity = config
        .identities
        .resolve(&repo_id)
        .expect("identities are configured, so there is a default one");
    let secrets = Secrets::load(ui)?;
    let token = secrets.api_token(&identity, host)?.ok_or_else(|| {
        format!(
            "The identity of {} ({}) has no API token for {host}. Run \
             `command gh` to use the accounts of gh instead",
            repo_id.display(config),
            identity.sources.join(", ")
        )
    })?;

    if host == GITHUB_HOST {
        command.env("GH_TOKEN", token);
    } else {
        command
            .env("GH_HOST", host)
            .env("GH_ENTERPRISE_TOKEN", token);
    }

    Err(Box::new(command.exec()))
}

/// Execute the `rt gh` command, with the arguments to give to `gh`.
pub fn run(ui: &Ui<'_>, config: &Config, args: &[OsString]) -> i32 {
    match exec_gh(ui, config, args) {
        Ok(()) => 0,
        Err(err) => {
            ui.error(err);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convert arguments for the tests.
    fn args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn repo_option_forms() {
        assert_eq!(
            repo_option(&args(&["pr", "list", "-R", "a/b"])),
            Some("a/b".to_string())
        );
        assert_eq!(repo_option(&args(&["-Ra/b"])), Some("a/b".to_string()));
        assert_eq!(
            repo_option(&args(&["--repo", "a/b"])),
            Some("a/b".to_string())
        );
        assert_eq!(
            repo_option(&args(&["--repo=a/b"])),
            Some("a/b".to_string())
        );
        assert_eq!(repo_option(&args(&["pr", "list"])), None);
        assert_eq!(repo_option(&args(&["api", "--", "-R", "a/b"])), None);
    }

    #[test]
    fn repo_id_forms() {
        let id = repo_id_from_option("a/b", "github.com").expect("valid");
        assert_eq!(id.name, "a/b");
        assert_eq!(id.remote.expect("remote").host_url, "github.com");

        let id = repo_id_from_option("ghe.company.com/a/b", "github.com")
            .expect("valid");
        assert_eq!(id.name, "a/b");
        assert_eq!(id.remote.expect("remote").host_url, "ghe.company.com");

        let id = repo_id_from_option("https://github.com/a/b.git", "x")
            .expect("valid");
        assert_eq!(id.name, "a/b");
        assert_eq!(id.remote.expect("remote").host_url, "github.com");

        assert!(repo_id_from_option("b", "github.com").is_none());
    }
}
