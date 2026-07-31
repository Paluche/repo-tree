//! Managements of http credentials
//! The idea is to have a better credential management supporting multiple
//! forge and potentially multiple identity per forge.
use std::fmt::Display;

use indoc::formatdoc;
use itertools::Itertools;
use serde::Deserialize;
use serde::Serialize;

use crate::error::ConfigError;
use crate::host::Remote;
use crate::repo_id::RepoId;

/// Signing configuration
#[derive(Serialize, Deserialize, Default)]
struct SigningConf {
    // TODO: Signing configuration
    /// Method used to sign the commits (GPG for instance).
    sign_method: String,

    /// Id of the key or key to use to do the signing,
    sign_key: String,
}

/// Scope on which an identity should be used.
#[derive(
    Serialize, Deserialize, Default, Hash, Eq, PartialEq, Ord, PartialOrd,
)]
struct Scope {
    /// Remote concerned by the scope. If None, then this is the global scope
    /// targeting all repositories from all forge if not specified otherwise by
    /// another rule.
    remote: Option<Remote>,

    /// Sub-directory within the forge reducing the scope, the smallest scope
    /// being a repository. Must be None if forge is None.
    path: Option<String>,
}

impl Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", toml::to_string(self).unwrap())
    }
}

impl Scope {
    /// Find out if a repository ID is within the scope.
    fn in_scope(&self, repo_id: &RepoId) -> bool {
        match (&self.remote, &self.path) {
            (None, None) => true,
            (None, Some(_)) => false, // Invalid scope.
            (Some(remote), None) => &repo_id.remote == remote,
            (Some(remote), Some(path)) => {
                &repo_id.remote == remote
                    && if path.ends_with("/") {
                        repo_id.name.starts_with(path)
                    } else {
                        &repo_id.name == path
                    }
            }
        }
    }
}

/// Default scopes array, defaulting to have the associated identity to be the
/// default, global identity.
fn default_global_scope() -> Vec<Scope> {
    Vec::from([Scope::default()])
}

/// User identity
#[derive(Serialize, Deserialize, Default)]
pub struct Identity {
    /// Scope on which thoses identities applies.
    #[serde(default = "default_global_scope")]
    pub scopes: Vec<Scope>,

    /// Name of the user authoring the commits
    pub name: String,

    /// E-mail of the user authoring the commits.
    pub email: String,

    /// Signing configuration to use for that user.
    pub signing: Option<SigningConf>,

    /// HTTPS credentials.
    pub credentials: Option<String>,
}

impl Identity {
    pub fn new(name: String, email: String) -> Identity {
        Identity {
            scopes: Vec::new(),
            name,
            email,
            signing: None,
            credentials: None,
        }
    }
}

impl Display for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

/// Configuration around the identities.
#[derive(Serialize, Deserialize, Default)]
pub struct Identities(Vec<Identity>);

impl Identities {
    fn scopes(&self) -> Vec<&Scope> {
        self.0.iter().flat_map(|id| &id.scopes).collect()
    }

    pub fn check(&self) -> Result<(), ConfigError> {
        let scopes = self.scopes();
        let mut global_config_found = false;

        for scope in scopes {
            if scope.remote.is_none() {
                if scope.path.is_none() {
                    global_config_found = true;
                } else {
                    return Err(ConfigError(formatdoc! {
                        "Invalid identity scope configuration:
                        {scope}
                        If the path is defined, the remote must be too."
                    }));
                }
            }
        }

        if !global_config_found {
            let global_scope = Scope {
                remote: None,
                path: None,
            };
            return Err(ConfigError(formatdoc! {
                "Missing identity with the global scope (default):
                {global_scope}"
            }));
        }

        // All scopes used in the configuration must be unique.
        for [scope_a, scope_b] in
            self.0.iter().flat_map(|id| &id.scopes).array_combinations()
        {
            if scope_a == scope_b {
                return Err(ConfigError(formatdoc! {
                    "Invalid identity scopes configuration, this scope is used several time
                    across the identities configuration:
                    {scope_a}"
                }));
            }
        }

        Ok(())
    }

    pub fn get(&self, repo_id: &RepoId) -> &Identity {
        let mut res: Option<(&Scope, &Identity)> = None;

        for id in self.0.iter() {
            for scope in id.scopes.iter() {
                if scope.in_scope(repo_id) {
                    if let Some((prev_scope, _)) = res {
                        if prev_scope < scope {
                            res = Some((scope, id));
                        }
                    } else {
                        res = Some((scope, id));
                    }
                }
            }
        }

        res.expect("Should have at lest one global identity").1
    }
}
