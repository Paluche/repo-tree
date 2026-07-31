//! Identities used in the repositories: who authors and signs the commits,
//! and which secrets authenticate to the forges.
//!
//! A `default` identity applies to every repository. The other identities
//! override part of it for the repositories within their scopes: a host, a
//! directory of a host, or a single repository.
use std::collections::BTreeMap;
use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;

use crate::error::ConfigError;
use crate::repo_id::RepoId;

/// Name of the identity applying to every repository.
pub const DEFAULT_IDENTITY: &str = "default";

/// Part of the repo tree an identity applies to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// Host of the repositories, as in their remote URL (e.g.
    /// `github.com`).
    pub host: String,

    /// Restrict the scope within the host. A path ending with `/` matches all
    /// the repositories whose name starts with it, any other path matches
    /// the repository with exactly that name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// How specific is a scope matching a repository. Ordered from the least to
/// the most specific.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Specificity {
    /// The scope matches the whole host.
    Host,
    /// The scope matches a directory of the host, with the length of the
    /// directory path, as longer paths are more specific.
    Directory(usize),
    /// The scope matches the repository exactly.
    Repository,
}

impl Scope {
    /// Find out how specifically the scope matches the repository, `None` if
    /// the repository is not in the scope.
    fn specificity(&self, repo_id: &RepoId) -> Option<Specificity> {
        let remote = repo_id.remote.as_ref()?;

        if remote.host_url != self.host {
            return None;
        }

        match &self.path {
            None => Some(Specificity::Host),
            Some(path) if path.ends_with('/') => repo_id
                .name
                .starts_with(path.as_str())
                .then_some(Specificity::Directory(path.len())),
            Some(path) => {
                (&repo_id.name == path).then_some(Specificity::Repository)
            }
        }
    }
}

impl Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.host)?;
        if let Some(path) = &self.path {
            write!(f, "/{path}")?;
        }
        Ok(())
    }
}

/// Program used to sign the commits.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SigningBackend {
    /// `gpg`, with PGP keys.
    Gpg,
    /// SSH keys.
    Ssh,
    /// `gpgsm`, with X.509 certificates.
    Gpgsm,
}

/// When the commits are signed.
#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum SignOn {
    /// Sign each commit when it is created or modified.
    #[default]
    Commit,
    /// Sign the commits only when they are pushed. Only Jujutsu supports it,
    /// Git signs at commit time.
    Push,
}

/// Keyword to disable the signing.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoSigning {
    /// Do not sign the commits.
    None,
}

/// Signing configuration of the commits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SigningKey {
    /// Program used to sign.
    pub backend: SigningBackend,

    /// Key to sign with, in the format the backend expects (key ID or
    /// fingerprint for `gpg`, path to the public key for SSH).
    pub key: String,

    /// When to sign the commits.
    #[serde(default)]
    pub on: SignOn,
}

/// Signing of the commits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Signing {
    /// Do not sign the commits (`signing = "none"`).
    Disabled(NoSigning),
    /// Sign the commits with this key.
    Enabled(SigningKey),
}

impl Display for Signing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled(NoSigning::None) => write!(f, "none"),
            Self::Enabled(signing) => {
                let backend = match signing.backend {
                    SigningBackend::Gpg => "gpg",
                    SigningBackend::Ssh => "ssh",
                    SigningBackend::Gpgsm => "gpgsm",
                };
                let on = match signing.on {
                    SignOn::Commit => "commit",
                    SignOn::Push => "push",
                };
                write!(f, "{backend} key {} on {on}", signing.key)
            }
        }
    }
}

/// Identifiers, in the secrets file, of the tokens used for a host.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Tokens {
    /// Token to fetch from and push to the repositories over HTTPS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,

    /// Token to access the forge API.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

impl Tokens {
    /// Override the tokens with the ones `other` defines.
    fn merge(&mut self, other: &Self) {
        if other.git.is_some() {
            self.git.clone_from(&other.git);
        }
        if other.api.is_some() {
            self.api.clone_from(&other.api);
        }
    }
}

/// An identity as configured. Every field is optional, as an identity only
/// defines what it overrides.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct IdentityConfig {
    /// Parts of the repo tree the identity applies to. Must be empty for the
    /// default identity, which applies everywhere.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<Scope>,

    /// Name of the author of the commits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// E-mail of the author of the commits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Signing of the commits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing: Option<Signing>,

    /// Tokens to use, per host.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, Tokens>,
}

/// Identity to use in a repository, once all the identities applying to it
/// are merged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedIdentity {
    /// Names of the identities merged, from the least to the most specific.
    pub sources: Vec<String>,

    /// Name of the author of the commits.
    pub name: String,

    /// E-mail of the author of the commits.
    pub email: String,

    /// Signing of the commits. `None` when no identity configures it, the
    /// signing configuration of the repository is then left untouched.
    pub signing: Option<Signing>,

    /// Tokens to use, per host.
    pub tokens: BTreeMap<String, Tokens>,
}

impl ResolvedIdentity {
    /// Override the identity with the fields `other` defines.
    fn merge(&mut self, name: &str, other: &IdentityConfig) {
        self.sources.push(name.to_string());
        if let Some(value) = &other.name {
            self.name.clone_from(value);
        }
        if let Some(value) = &other.email {
            self.email.clone_from(value);
        }
        if other.signing.is_some() {
            self.signing.clone_from(&other.signing);
        }
        for (host, tokens) in &other.tokens {
            self.tokens.entry(host.clone()).or_default().merge(tokens);
        }
    }
}

impl Display for ResolvedIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

/// The configured identities, by name.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(transparent)]
pub struct Identities(BTreeMap<String, IdentityConfig>);

impl Identities {
    /// Find out if no identity is configured, in which case repo-tree does not
    /// manage the identities.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check the identities configuration is consistent.
    pub fn check(&self) -> Result<(), ConfigError> {
        if self.is_empty() {
            return Ok(());
        }

        let Some(default) = self.0.get(DEFAULT_IDENTITY) else {
            return Err(ConfigError(format!(
                "Missing the \"{DEFAULT_IDENTITY}\" identity, applying to all \
                 the repositories"
            )));
        };

        if !default.scopes.is_empty() {
            return Err(ConfigError(format!(
                "The \"{DEFAULT_IDENTITY}\" identity applies to all the \
                 repositories, it cannot have scopes"
            )));
        }

        if default.name.is_none() || default.email.is_none() {
            return Err(ConfigError(format!(
                "The \"{DEFAULT_IDENTITY}\" identity must define a name and \
                 an email"
            )));
        }

        let mut scopes: Vec<(&str, &Scope)> = Vec::new();

        for (name, identity) in &self.0 {
            if name == DEFAULT_IDENTITY {
                continue;
            }

            if identity.scopes.is_empty() {
                return Err(ConfigError(format!(
                    "The identity \"{name}\" has no scopes, it applies to no \
                     repository"
                )));
            }

            for scope in &identity.scopes {
                if scope.host.is_empty() {
                    return Err(ConfigError(format!(
                        "The identity \"{name}\" has a scope with an empty \
                         host"
                    )));
                }

                if let Some(path) = &scope.path
                    && (path.is_empty() || path.starts_with('/'))
                {
                    return Err(ConfigError(format!(
                        "The identity \"{name}\" has the scope {scope} whose \
                         path is empty or starts with \"/\""
                    )));
                }

                if let Some((other, _)) =
                    scopes.iter().find(|(_, other)| *other == scope)
                {
                    return Err(ConfigError(format!(
                        "The scope {scope} is used by both the \"{other}\" \
                         and the \"{name}\" identities"
                    )));
                }

                scopes.push((name, scope));
            }
        }

        Ok(())
    }

    /// Get the identity to use in a repository. `None` when no identity is
    /// configured.
    pub fn resolve(&self, repo_id: &RepoId) -> Option<ResolvedIdentity> {
        let default = self.0.get(DEFAULT_IDENTITY)?;

        let mut ret = ResolvedIdentity {
            sources: Vec::new(),
            name: String::new(),
            email: String::new(),
            signing: None,
            tokens: BTreeMap::new(),
        };
        ret.merge(DEFAULT_IDENTITY, default);

        let mut matching: Vec<(Specificity, &str, &IdentityConfig)> = self
            .0
            .iter()
            .filter(|(name, _)| name.as_str() != DEFAULT_IDENTITY)
            .filter_map(|(name, identity)| {
                identity
                    .scopes
                    .iter()
                    .filter_map(|scope| scope.specificity(repo_id))
                    .max()
                    .map(|specificity| (specificity, name.as_str(), identity))
            })
            .collect();
        // Scopes are unique, so two identities cannot match with the same
        // specificity, except two directories of the same length, which
        // cannot both contain the repository.
        matching.sort_by(|a, b| a.0.cmp(&b.0));

        for (_, name, identity) in matching {
            ret.merge(name, identity);
        }

        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identities configuration used by the tests.
    const CONFIG: &str = r#"
        [default]
        name = "Default Name"
        email = "default@example.com"
        signing = { backend = "gpg", key = "DEFAULT" }
        tokens."github.com" = { git = "github-default", api = "github-api" }

        [work]
        email = "work@company.com"
        scopes = [
            { host = "github.com", path = "company/" },
            { host = "gitlab.company.com" },
        ]
        tokens."github.com" = { git = "github-work" }

        [team]
        name = "Team Name"
        scopes = [{ host = "github.com", path = "company/team/" }]

        [legacy]
        signing = "none"
        scopes = [{ host = "github.com", path = "company/team/legacy" }]
    "#;

    /// Parse and check the test configuration.
    fn identities() -> Identities {
        let identities: Identities =
            toml::from_str(CONFIG).expect("test configuration is valid TOML");
        identities
            .check()
            .expect("test configuration is consistent");
        identities
    }

    /// Resolve the identity of a GitHub repository.
    fn resolve(url: &str) -> ResolvedIdentity {
        identities()
            .resolve(&RepoId::from_remote_url(url).expect("valid URL"))
            .expect("identities are configured")
    }

    #[test]
    fn default_identity() {
        let identity = resolve("https://github.com/someone/repo.git");

        assert_eq!(identity.sources, ["default"]);
        assert_eq!(identity.name, "Default Name");
        assert_eq!(identity.email, "default@example.com");
        assert_eq!(
            identity.signing,
            Some(Signing::Enabled(SigningKey {
                backend: SigningBackend::Gpg,
                key: "DEFAULT".to_string(),
                on: SignOn::Commit,
            }))
        );
    }

    #[test]
    fn local_repository_uses_default() {
        let repo_id = RepoId::from_repo(&"/tmp/repo", None).expect("no URL");
        let identity = identities()
            .resolve(&repo_id)
            .expect("identities are configured");

        assert_eq!(identity.sources, ["default"]);
    }

    #[test]
    fn host_scope() {
        let identity = resolve("git@gitlab.company.com:any/repo.git");

        assert_eq!(identity.sources, ["default", "work"]);
        assert_eq!(identity.name, "Default Name");
        assert_eq!(identity.email, "work@company.com");
    }

    #[test]
    fn directory_scope_merges_tokens_per_field() {
        let identity = resolve("https://github.com/company/repo.git");

        assert_eq!(identity.sources, ["default", "work"]);
        assert_eq!(
            identity.tokens["github.com"],
            Tokens {
                git: Some("github-work".to_string()),
                api: Some("github-api".to_string()),
            }
        );
    }

    #[test]
    fn directory_scope_is_a_prefix() {
        let identity = resolve("https://github.com/company-other/repo.git");

        assert_eq!(identity.sources, ["default"]);
    }

    #[test]
    fn most_specific_scope_applies_last() {
        let identity = resolve("https://github.com/company/team/legacy.git");

        assert_eq!(identity.sources, ["default", "work", "team", "legacy"]);
        assert_eq!(identity.name, "Team Name");
        assert_eq!(identity.email, "work@company.com");
        assert_eq!(identity.signing, Some(Signing::Disabled(NoSigning::None)));
    }

    #[test]
    fn repository_scope_is_exact() {
        let identity =
            resolve("https://github.com/company/team/legacy-other.git");

        assert_eq!(identity.sources, ["default", "work", "team"]);
    }

    #[test]
    fn no_identity_configured() {
        let identities = Identities::default();
        let repo_id = RepoId::from_remote_url("https://github.com/a/b.git")
            .expect("valid URL");

        assert!(identities.check().is_ok());
        assert!(identities.resolve(&repo_id).is_none());
    }

    /// Check that a configuration is rejected with the expected message.
    fn check_error(config: &str, expected: &str) {
        let identities: Identities =
            toml::from_str(config).expect("test configuration is valid TOML");
        let err = identities.check().expect_err("configuration is invalid");

        assert!(
            err.0.contains(expected),
            "\"{}\" does not contain \"{expected}\"",
            err.0
        );
    }

    #[test]
    fn missing_default() {
        check_error(
            r#"
            [work]
            name = "Name"
            scopes = [{ host = "github.com" }]
            "#,
            "Missing the \"default\" identity",
        );
    }

    #[test]
    fn default_with_scopes() {
        check_error(
            r#"
            [default]
            name = "Name"
            email = "email"
            scopes = [{ host = "github.com" }]
            "#,
            "it cannot have scopes",
        );
    }

    #[test]
    fn default_without_email() {
        check_error(
            r#"
            [default]
            name = "Name"
            "#,
            "must define a name and an email",
        );
    }

    #[test]
    fn identity_without_scopes() {
        check_error(
            r#"
            [default]
            name = "Name"
            email = "email"

            [work]
            email = "work"
            "#,
            "\"work\" has no scopes",
        );
    }

    #[test]
    fn invalid_path() {
        check_error(
            r#"
            [default]
            name = "Name"
            email = "email"

            [work]
            scopes = [{ host = "github.com", path = "/company/" }]
            "#,
            "path is empty or starts with",
        );
    }

    #[test]
    fn duplicated_scope() {
        check_error(
            r#"
            [default]
            name = "Name"
            email = "email"

            [a]
            scopes = [{ host = "github.com", path = "company/" }]

            [b]
            scopes = [{ host = "github.com", path = "company/" }]
            "#,
            "used by both the \"a\" and the \"b\" identities",
        );
    }

    #[test]
    fn unknown_field() {
        let result = toml::from_str::<Identities>(
            r#"
            [default]
            name = "Name"
            email = "email"
            scope = [{ host = "github.com" }]
            "#,
        );

        assert!(result.is_err());
    }
}
