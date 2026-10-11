//! Credentials: secrets whose keychain account is decided at run time.
//!
//! [`SecretName`](crate::SecretName) is a closed set, one item per app-wide
//! key. An integration has one credential per connected account (INT-00,
//! ADR 0029), so its items are named by a validated [`CredentialKey`]
//! (`integration.slack.<connection id>`) and kept through the
//! [`CredentialStore`] port. Nothing here is ever reachable from the UI.

use async_trait::async_trait;

use crate::{SecretError, SecretValue};

/// The longest credential, in characters: an OAuth access token (Microsoft's
/// run to several KB) and its refresh token, stored together as one item.
pub const MAX_CREDENTIAL_CHARS: usize = 16 * 1024;

/// Every credential account starts with this, so they never collide with a
/// [`SecretName`](crate::SecretName) account.
const PREFIX: &str = "integration.";

/// The longest credential account name.
const MAX_KEY_CHARS: usize = 128;

/// The keychain account of one credential: `integration.` followed by
/// lowercase ASCII letters, digits, `.`, `_` and `-`, at most 128 characters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CredentialKey(String);

impl CredentialKey {
    /// Validates `account` as a credential key.
    pub fn parse(account: &str) -> Result<Self, SecretError> {
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || ".-_".contains(c);
        let rest = account.strip_prefix(PREFIX).unwrap_or_default();
        if rest.is_empty() || account.len() > MAX_KEY_CHARS || !rest.chars().all(allowed) {
            return Err(SecretError::InvalidCredentialKey);
        }
        Ok(Self(account.to_owned()))
    }

    /// The keychain account name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where credentials live.
///
/// Implementations honour the same contract as
/// [`SecretStore`](crate::SecretStore), checked by `contract_tests.rs`:
/// `put` replaces, `read` of a missing key is `Ok(None)`, `remove` of a
/// missing key succeeds, and an item the OS refuses to hand over fails with
/// [`SecretError::CredentialDenied`].
#[async_trait]
pub trait CredentialStore: Send + Sync {
    /// Saves `value` under `key`, replacing any saved value in one write.
    async fn put(&self, key: &CredentialKey, value: SecretValue) -> Result<(), SecretError>;

    /// The saved value, or `None` when nothing is saved.
    async fn read(&self, key: &CredentialKey) -> Result<Option<SecretValue>, SecretError>;

    /// Forgets the saved value; succeeds when nothing is saved.
    async fn remove(&self, key: &CredentialKey) -> Result<(), SecretError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_namespaced_lowercase_account_names() {
        let key = CredentialKey::parse("integration.slack.0192-ab_c").unwrap();
        assert_eq!(key.as_str(), "integration.slack.0192-ab_c");

        for bad in [
            "",
            "integration.",
            "slack.token",
            "openai_api_key",
            "integration.Slack",
            "integration.a b",
            "integration.a/b",
        ] {
            assert!(CredentialKey::parse(bad).is_err(), "{bad:?}");
        }
        let long = format!("integration.{}", "a".repeat(MAX_KEY_CHARS));
        assert!(CredentialKey::parse(&long).is_err());
    }
}
