//! Secrets used to authenticate to the forges. They are stored apart from
//! the configuration, in `secrets.toml` next to `config.toml`, so that the
//! configuration can be shared and the secrets protected on their own.
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Debug;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

use crate::config::ResolvedIdentity;
use crate::config::config_dir;
use crate::forge::Forge;
use crate::ui::Ui;
use crate::ui_warning;

#[derive(Debug, Error)]
#[error("Missing the secret \"{0}\" in {1}")]
/// A secret referenced by the configuration is not in the secrets file.
pub struct MissingSecretError(pub String, pub String);

/// A token, with the username to authenticate with it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Secret {
    /// Username to authenticate with. Defaults to the one the forge expects
    /// with a token, if it has one.
    #[serde(default)]
    pub username: Option<String>,

    /// The token.
    pub token: String,
}

impl Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Secret")
            .field("username", &self.username)
            .field("token", &"<redacted>")
            .finish()
    }
}

/// Credentials for Git to fetch and push over HTTPS.
#[derive(PartialEq, Eq)]
pub struct GitCredential<'secrets> {
    /// Username to authenticate with.
    pub username: &'secrets str,
    /// Token to use as password.
    pub password: &'secrets str,
}

impl Debug for GitCredential<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitCredential")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

/// The secrets, by identifier.
#[derive(Deserialize, Default)]
#[serde(transparent)]
pub struct Secrets(BTreeMap<String, Secret>);

impl Secrets {
    /// Path to the secrets file.
    pub fn path() -> Result<PathBuf, Box<dyn Error>> {
        Ok(config_dir()?.join("secrets.toml"))
    }

    /// Load the secrets file, warning when other users can read it.
    pub fn load(ui: &Ui<'_>) -> Result<Self, Box<dyn Error>> {
        let path = Self::path()?;

        if !path.is_file() {
            return Ok(Self::default());
        }

        let mode = fs::metadata(&path)?.permissions().mode();
        if mode & 0o077 != 0 {
            ui_warning!(
                ui,
                "{} can be accessed by other users, restrict it with chmod 600",
                path.display()
            );
        }

        toml::from_str(&fs::read_to_string(&path)?).map_err(|err| {
            format!("Unable to parse {}: {err}", path.display()).into()
        })
    }

    /// Get a secret from its identifier.
    fn get(&self, id: &str) -> Result<&Secret, Box<dyn Error>> {
        self.0.get(id).ok_or_else(|| {
            let path = Self::path()
                .map_or("the secrets file".to_string(), |p| {
                    p.display().to_string()
                });
            MissingSecretError(id.to_string(), path).into()
        })
    }

    /// Get the credentials Git should use for the host with the identity.
    /// `None` when the identity has no Git token for the host.
    pub fn git_credential<'secrets>(
        &'secrets self,
        identity: &ResolvedIdentity,
        host: &str,
        forge: Option<&Forge>,
    ) -> Result<Option<GitCredential<'secrets>>, Box<dyn Error>> {
        let Some(id) = identity.tokens.get(host).and_then(|t| t.git.as_ref())
        else {
            return Ok(None);
        };
        let secret = self.get(id)?;
        let username = match &secret.username {
            Some(username) => username.as_str(),
            None => forge.and_then(Forge::token_username).ok_or_else(|| {
                format!(
                    "The secret \"{id}\" needs a username: the forge of \
                     {host} has no default username for tokens"
                )
            })?,
        };

        Ok(Some(GitCredential {
            username,
            password: &secret.token,
        }))
    }

    /// Get the token to access the API of the forge of the host with the
    /// identity. `None` when the identity has no API token for the host.
    pub fn api_token<'secrets>(
        &'secrets self,
        identity: &ResolvedIdentity,
        host: &str,
    ) -> Result<Option<&'secrets str>, Box<dyn Error>> {
        let Some(id) = identity.tokens.get(host).and_then(|t| t.api.as_ref())
        else {
            return Ok(None);
        };

        Ok(Some(&self.get(id)?.token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Tokens;

    /// Parse the secrets used by the tests.
    fn secrets() -> Secrets {
        toml::from_str(
            r#"
            [github]
            token = "github-token"

            [forgejo]
            username = "me"
            token = "forgejo-token"

            [anonymous]
            token = "anonymous-token"
            "#,
        )
        .expect("valid secrets")
    }

    /// Build an identity using the given Git token for the host.
    fn identity(host: &str, git: &str) -> ResolvedIdentity {
        ResolvedIdentity {
            sources: vec!["default".to_string()],
            name: "Name".to_string(),
            email: "name@example.com".to_string(),
            signing: None,
            tokens: BTreeMap::from([(
                host.to_string(),
                Tokens {
                    git: Some(git.to_string()),
                    api: None,
                },
            )]),
        }
    }

    #[test]
    fn forge_default_username() {
        let secrets = secrets();
        let credential = secrets
            .git_credential(
                &identity("github.com", "github"),
                "github.com",
                Some(&Forge::GitHub),
            )
            .expect("secret exists");

        assert_eq!(
            credential,
            Some(GitCredential {
                username: "x-access-token",
                password: "github-token",
            })
        );
    }

    #[test]
    fn explicit_username() {
        let secrets = secrets();
        let credential = secrets
            .git_credential(
                &identity("codeberg.org", "forgejo"),
                "codeberg.org",
                Some(&Forge::Forgejo),
            )
            .expect("secret exists");

        assert_eq!(
            credential,
            Some(GitCredential {
                username: "me",
                password: "forgejo-token",
            })
        );
    }

    #[test]
    fn token_only_for_its_host() {
        let secrets = secrets();
        let credential = secrets
            .git_credential(
                &identity("github.com", "github"),
                "gitlab.com",
                Some(&Forge::GitLab),
            )
            .expect("no secret needed");

        assert_eq!(credential, None);
    }

    #[test]
    fn missing_username() {
        let secrets = secrets();
        let result = secrets.git_credential(
            &identity("git.example.com", "anonymous"),
            "git.example.com",
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn missing_secret() {
        let secrets = secrets();
        let result = secrets.git_credential(
            &identity("github.com", "unknown"),
            "github.com",
            Some(&Forge::GitHub),
        );

        assert!(
            result
                .err()
                .is_some_and(|err| err.to_string().contains("\"unknown\""))
        );
    }

    #[test]
    fn api_token() {
        let secrets = secrets();
        let mut identity = identity("github.com", "unknown");
        identity
            .tokens
            .get_mut("github.com")
            .expect("tokens for github.com")
            .api = Some("github".to_string());

        assert_eq!(
            secrets
                .api_token(&identity, "github.com")
                .expect("secret exists"),
            Some("github-token")
        );
        assert_eq!(
            secrets
                .api_token(&identity, "gitlab.com")
                .expect("no secret needed"),
            None
        );
    }

    #[test]
    fn token_is_not_printed() {
        assert!(
            !format!("{:?}", secrets().get("github")).contains("github-token")
        );
    }
}
