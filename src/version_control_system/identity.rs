//! Translation of an identity into the configuration of the version control
//! systems.
use std::fmt::Display;

use crate::config::ResolvedIdentity;
use crate::config::SignOn;
use crate::config::Signing;
use crate::config::SigningBackend;

/// Jujutsu revset of the commits to sign on push: our own unsigned commits.
const JJ_SIGN_ON_PUSH: &str = "~signed() & mine()";

/// Jujutsu revset to sign no commit on push.
const JJ_NO_SIGN_ON_PUSH: &str = "none()";

/// A configuration value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    /// Text value.
    String(String),
    /// Boolean value.
    Bool(bool),
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(value) => write!(f, "{value:?}"),
            Self::Bool(value) => write!(f, "{value}"),
        }
    }
}

/// A configuration value of a repository which differs from the identity it
/// should use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigChange {
    /// Version control system the configuration belongs to (`jj` or `git`).
    pub vcs: &'static str,
    /// Configuration key.
    pub key: &'static str,
    /// Value currently set in the repository configuration, if any.
    pub current: Option<ConfigValue>,
    /// Value the identity requires.
    pub expected: ConfigValue,
}

impl Display for ConfigChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: ", self.vcs, self.key)?;
        match &self.current {
            Some(current) => write!(f, "{current}")?,
            None => write!(f, "unset")?,
        }
        write!(f, " -> {}", self.expected)
    }
}

/// Shortcut to build a text configuration value.
fn string(value: &str) -> ConfigValue {
    ConfigValue::String(value.to_string())
}

/// Get the Jujutsu configuration the identity requires.
pub fn jujutsu_settings(
    identity: &ResolvedIdentity,
) -> Vec<(&'static str, ConfigValue)> {
    let mut ret = vec![
        ("user.name", string(&identity.name)),
        ("user.email", string(&identity.email)),
    ];

    match &identity.signing {
        None => (),
        Some(Signing::Disabled(_)) => {
            ret.push(("signing.behavior", string("drop")));
            ret.push(("revsets.git-sign-on-push", string(JJ_NO_SIGN_ON_PUSH)));
        }
        Some(Signing::Enabled(signing)) => {
            let (behavior, sign_on_push) = match signing.on {
                SignOn::Commit => ("own", JJ_NO_SIGN_ON_PUSH),
                SignOn::Push => ("drop", JJ_SIGN_ON_PUSH),
            };
            ret.push(("signing.behavior", string(behavior)));
            ret.push((
                "signing.backend",
                string(match signing.backend {
                    SigningBackend::Gpg => "gpg",
                    SigningBackend::Ssh => "ssh",
                    SigningBackend::Gpgsm => "gpgsm",
                }),
            ));
            ret.push(("signing.key", string(&signing.key)));
            ret.push(("revsets.git-sign-on-push", string(sign_on_push)));
        }
    }

    ret
}

/// Get the Git configuration the identity requires. Git has no signing on
/// push, the commits are then signed at commit time.
pub fn git_settings(
    identity: &ResolvedIdentity,
) -> Vec<(&'static str, ConfigValue)> {
    let mut ret = vec![
        ("user.name", string(&identity.name)),
        ("user.email", string(&identity.email)),
    ];

    match &identity.signing {
        None => (),
        Some(Signing::Disabled(_)) => {
            ret.push(("commit.gpgsign", ConfigValue::Bool(false)));
        }
        Some(Signing::Enabled(signing)) => {
            ret.push(("commit.gpgsign", ConfigValue::Bool(true)));
            ret.push((
                "gpg.format",
                string(match signing.backend {
                    SigningBackend::Gpg => "openpgp",
                    SigningBackend::Ssh => "ssh",
                    SigningBackend::Gpgsm => "x509",
                }),
            ));
            ret.push(("user.signingkey", string(&signing.key)));
        }
    }

    ret
}

/// Compare the settings an identity requires with the values `current`
/// reads from the repository configuration. `current` gets the key and the
/// expected value, to read the value with the same type.
pub fn changes<F>(
    vcs: &'static str,
    settings: Vec<(&'static str, ConfigValue)>,
    mut current: F,
) -> Vec<ConfigChange>
where
    F: FnMut(&str, &ConfigValue) -> Option<ConfigValue>,
{
    settings
        .into_iter()
        .filter_map(|(key, expected)| {
            let current = current(key, &expected);
            (current.as_ref() != Some(&expected)).then_some(ConfigChange {
                vcs,
                key,
                current,
                expected,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::config::SigningKey;

    /// Build an identity with the given signing configuration.
    fn identity(signing: Option<Signing>) -> ResolvedIdentity {
        ResolvedIdentity {
            sources: vec!["default".to_string()],
            name: "Name".to_string(),
            email: "name@example.com".to_string(),
            signing,
            tokens: BTreeMap::new(),
        }
    }

    /// Build a signing configuration.
    fn signing(backend: SigningBackend, on: SignOn) -> Signing {
        Signing::Enabled(SigningKey {
            backend,
            key: "KEY".to_string(),
            on,
        })
    }

    /// Format the settings to compare them easily.
    fn format(settings: &[(&'static str, ConfigValue)]) -> Vec<String> {
        settings.iter().map(|(k, v)| format!("{k}={v}")).collect()
    }

    #[test]
    fn signing_not_managed() {
        assert_eq!(
            format(&jujutsu_settings(&identity(None))),
            ["user.name=\"Name\"", "user.email=\"name@example.com\""]
        );
        assert_eq!(
            format(&git_settings(&identity(None))),
            ["user.name=\"Name\"", "user.email=\"name@example.com\""]
        );
    }

    #[test]
    fn signing_disabled() {
        let identity =
            identity(Some(Signing::Disabled(crate::config::NoSigning::None)));

        assert_eq!(
            format(&jujutsu_settings(&identity)[2..]),
            [
                "signing.behavior=\"drop\"",
                "revsets.git-sign-on-push=\"none()\""
            ]
        );
        assert_eq!(
            format(&git_settings(&identity)[2..]),
            ["commit.gpgsign=false"]
        );
    }

    #[test]
    fn sign_on_commit() {
        let identity =
            identity(Some(signing(SigningBackend::Ssh, SignOn::Commit)));

        assert_eq!(
            format(&jujutsu_settings(&identity)[2..]),
            [
                "signing.behavior=\"own\"",
                "signing.backend=\"ssh\"",
                "signing.key=\"KEY\"",
                "revsets.git-sign-on-push=\"none()\"",
            ]
        );
        assert_eq!(
            format(&git_settings(&identity)[2..]),
            [
                "commit.gpgsign=true",
                "gpg.format=\"ssh\"",
                "user.signingkey=\"KEY\""
            ]
        );
    }

    #[test]
    fn sign_on_push() {
        let identity =
            identity(Some(signing(SigningBackend::Gpgsm, SignOn::Push)));

        assert_eq!(
            format(&jujutsu_settings(&identity)[2..]),
            [
                "signing.behavior=\"drop\"",
                "signing.backend=\"gpgsm\"",
                "signing.key=\"KEY\"",
                "revsets.git-sign-on-push=\"~signed() & mine()\"",
            ]
        );
        assert_eq!(
            format(&git_settings(&identity)[2..]),
            [
                "commit.gpgsign=true",
                "gpg.format=\"x509\"",
                "user.signingkey=\"KEY\""
            ]
        );
    }

    #[test]
    fn only_differences_are_changes() {
        let changes =
            changes("jj", jujutsu_settings(&identity(None)), |key, _| {
                (key == "user.name").then(|| string("Name"))
            });

        assert_eq!(
            changes,
            [ConfigChange {
                vcs: "jj",
                key: "user.email",
                current: None,
                expected: string("name@example.com"),
            }]
        );
    }
}
