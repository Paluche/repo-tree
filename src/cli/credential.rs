//! Git credential helper, giving Git the token of the identity of the
//! repository it fetches from or pushes to.
//!
//! See <https://git-scm.com/docs/gitcredentials> for the protocol.
use std::error::Error;
use std::io::BufRead;
use std::io::Write;

use clap::Args;
use clap::ValueEnum;

use crate::cli::get_cwd;
use crate::config::Config;
use crate::repo_id::Remote;
use crate::repo_id::RepoId;
use crate::repository::Repository;
use crate::secrets::GitCredential;
use crate::secrets::Secrets;
use crate::ui::Ui;

/// Operations Git asks a credential helper for.
#[derive(Clone, ValueEnum)]
enum Operation {
    /// Get the credentials.
    Get,
    /// Store credentials which worked. rt does not store anything.
    Store,
    /// Erase credentials which did not work. rt does not erase anything.
    Erase,
}

/// Git credential helper: give Git the token of the identity of the
/// repository. Called by Git, see `rt identity setup-git`.
#[derive(Args)]
pub struct CredentialArgs {
    /// Operation Git asks for.
    operation: Operation,
}

/// Attributes of a credential request relevant to find the identity.
#[derive(Debug, Default, PartialEq, Eq)]
struct Request {
    /// Protocol (`https`).
    protocol: Option<String>,
    /// Host, with the port if any.
    host: Option<String>,
    /// Path of the repository, given when `credential.useHttpPath` is set.
    path: Option<String>,
}

impl Request {
    /// Read a request: `key=value` lines up to an empty line or the end of
    /// the input.
    fn read<R: BufRead>(input: R) -> Result<Self, Box<dyn Error>> {
        let mut ret = Self::default();

        for line in input.lines() {
            let line = line?;
            if line.is_empty() {
                break;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(
                    format!("Invalid credential request: {line}").into()
                );
            };
            let value = Some(value.to_string());
            match key {
                "protocol" => ret.protocol = value,
                "host" => ret.host = value,
                "path" => ret.path = value,
                _ => (),
            }
        }

        Ok(ret)
    }

    /// Get the identifier of the repository the request is for. Without the
    /// path, use the repository of the current directory if it is on the
    /// same host, otherwise the host only.
    fn repo_id(&self, ui: &Ui<'_>, config: &Config) -> Option<RepoId> {
        let protocol = self.protocol.as_deref()?;
        let host = self.host.as_deref()?;

        if let Some(path) = &self.path {
            return RepoId::from_remote_url(&format!(
                "{protocol}://{host}/{path}"
            ))
            .ok();
        }

        let host_url = host.split_once(':').map_or(host, |(h, _)| h);

        if let Ok(repository) =
            Repository::discover_silent(config, &get_cwd(ui))
            && repository
                .id
                .remote
                .as_ref()
                .is_some_and(|r| r.host_url == host_url)
        {
            return Some(repository.id);
        }

        Some(RepoId {
            remote: Some(Remote {
                url: format!("{protocol}://{host}"),
                host_url: host_url.to_string(),
            }),
            name: String::new(),
        })
    }
}

/// Write the credentials in answer to a `get` request.
fn write_credential<W: Write>(
    mut output: W,
    credential: &GitCredential<'_>,
) -> Result<(), Box<dyn Error>> {
    if credential.username.contains('\n') || credential.password.contains('\n')
    {
        return Err("Credentials cannot contain a new line".into());
    }

    write!(
        output,
        "username={}\npassword={}\n",
        credential.username, credential.password
    )?;
    Ok(())
}

/// Answer a `get` request.
fn get(ui: &Ui<'_>, config: &Config) -> Result<(), Box<dyn Error>> {
    let request = Request::read(std::io::stdin().lock())?;

    if request.protocol.as_deref() != Some("https") {
        return Ok(());
    }

    let Some(repo_id) = request.repo_id(ui, config) else {
        return Ok(());
    };
    let Some(identity) = config.identities.resolve(&repo_id) else {
        return Ok(());
    };
    let Some(remote) = &repo_id.remote else {
        return Ok(());
    };
    let forge = config
        .get_remote_host(&remote.host_url)
        .and_then(|h| h.info.forge.as_ref());
    let secrets = Secrets::load(ui)?;

    if let Some(credential) =
        secrets.git_credential(&identity, &remote.host_url, forge)?
    {
        write_credential(std::io::stdout().lock(), &credential)?;
    }

    Ok(())
}

/// Execute the `rt credential` command.
pub fn run(ui: &Ui<'_>, config: &Config, args: &CredentialArgs) -> i32 {
    match args.operation {
        Operation::Get => match get(ui, config) {
            Ok(()) => 0,
            Err(err) => {
                ui.error(err);
                1
            }
        },
        // Git informs the helpers of the result of the authentication. The
        // tokens are managed in the secrets file, there is nothing to do.
        Operation::Store | Operation::Erase => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_request() {
        let input = concat!(
            "protocol=https\n",
            "host=github.com\n",
            "path=company/repo.git\n",
            "wwwauth[]=Basic\n",
            "\n",
            "ignored=after\n",
        );
        let request = Request::read(input.as_bytes()).expect("valid request");

        assert_eq!(
            request,
            Request {
                protocol: Some("https".to_string()),
                host: Some("github.com".to_string()),
                path: Some("company/repo.git".to_string()),
            }
        );
    }

    #[test]
    fn invalid_request() {
        assert!(Request::read("protocol https\n".as_bytes()).is_err());
    }

    #[test]
    fn repo_id_from_path() {
        let config = Config::test_default();
        let ui = Ui::new(&config);
        let request = Request {
            protocol: Some("https".to_string()),
            host: Some("github.com:443".to_string()),
            path: Some("company/repo.git".to_string()),
        };
        let repo_id = request.repo_id(&ui, &config).expect("valid request");

        assert_eq!(repo_id.name, "company/repo");
        assert_eq!(
            repo_id.remote.expect("has a remote").host_url,
            "github.com"
        );
    }

    #[test]
    fn credential_output() {
        let mut output = Vec::new();
        write_credential(
            &mut output,
            &GitCredential {
                username: "user",
                password: "token",
            },
        )
        .expect("valid credential");

        assert_eq!(output, b"username=user\npassword=token\n");
    }

    #[test]
    fn credential_with_new_line() {
        let result = write_credential(
            Vec::new(),
            &GitCredential {
                username: "user",
                password: "token\nhost=evil.com",
            },
        );

        assert!(result.is_err());
    }
}
