//! [`SecretStore`] and [`CredentialStore`] over the OS keychain, through the
//! `keyring` crate.
//!
//! Each secret is a generic password with the service set to the app's bundle
//! identifier (`com.xenshreyas.deino`) and the account set to
//! [`SecretName::account`] or [`CredentialKey::as_str`]. Keychain calls
//! block, sometimes on a system prompt, so each runs on Tokio's blocking pool.

use async_trait::async_trait;

use crate::error::PlatformError;
use crate::{CredentialKey, CredentialStore, SecretError, SecretName, SecretStore, SecretValue};

/// Secrets in the macOS Keychain (the platform's native store elsewhere).
#[derive(Clone, Debug)]
pub struct KeychainSecretStore {
    service: String,
}

/// One keychain item: its account, and who to blame when access is denied.
#[derive(Clone, Debug)]
enum Item {
    /// An app-wide key.
    Named(SecretName),
    /// An integration credential.
    Credential(CredentialKey),
}

impl Item {
    fn account(&self) -> &str {
        match self {
            Self::Named(name) => name.account(),
            Self::Credential(key) => key.as_str(),
        }
    }

    fn denied(&self) -> SecretError {
        match self {
            Self::Named(name) => SecretError::AccessDenied(*name),
            Self::Credential(_) => SecretError::CredentialDenied,
        }
    }

    /// Validates a stored password; a blank item (written by another tool)
    /// counts as nothing saved.
    fn value(&self, password: &str) -> Option<SecretValue> {
        match self {
            Self::Named(_) => SecretValue::parse(password).ok(),
            Self::Credential(_) => SecretValue::parse_credential(password).ok(),
        }
    }
}

impl KeychainSecretStore {
    /// A store whose items carry `service`: the app's bundle identifier, so
    /// a build with another identifier never sees this app's keys.
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    /// Runs `operation` on the keychain entry for `item`, off the async runtime.
    async fn with_entry<T, F>(&self, item: &Item, operation: F) -> Result<T, SecretError>
    where
        T: Send + 'static,
        F: FnOnce(&keyring::Entry) -> keyring::Result<T> + Send + 'static,
    {
        let service = self.service.clone();
        let account = item.account().to_owned();
        let outcome = tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, &account)?;
            operation(&entry)
        })
        .await
        .map_err(|join| SecretError::Backend(Box::new(join)))?;
        outcome.map_err(|error| classify(item, error))
    }

    async fn write(&self, item: &Item, value: SecretValue) -> Result<(), SecretError> {
        self.with_entry(item, move |entry| entry.set_password(value.expose()))
            .await
    }

    async fn read_item(&self, item: &Item) -> Result<Option<SecretValue>, SecretError> {
        let stored = self
            .with_entry(item, |entry| match entry.get_password() {
                Ok(password) => Ok(Some(zeroize::Zeroizing::new(password))),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(error) => Err(error),
            })
            .await?;
        Ok(stored.and_then(|password| item.value(&password)))
    }

    async fn erase(&self, item: &Item) -> Result<(), SecretError> {
        self.with_entry(item, |entry| match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error),
        })
        .await
    }
}

#[async_trait]
impl SecretStore for KeychainSecretStore {
    async fn set(&self, name: SecretName, value: SecretValue) -> Result<(), SecretError> {
        self.write(&Item::Named(name), value).await
    }

    async fn get(&self, name: SecretName) -> Result<Option<SecretValue>, SecretError> {
        self.read_item(&Item::Named(name)).await
    }

    async fn delete(&self, name: SecretName) -> Result<(), SecretError> {
        self.erase(&Item::Named(name)).await
    }
}

#[async_trait]
impl CredentialStore for KeychainSecretStore {
    async fn put(&self, key: &CredentialKey, value: SecretValue) -> Result<(), SecretError> {
        self.write(&Item::Credential(key.clone()), value).await
    }

    async fn read(&self, key: &CredentialKey) -> Result<Option<SecretValue>, SecretError> {
        self.read_item(&Item::Credential(key.clone())).await
    }

    async fn remove(&self, key: &CredentialKey) -> Result<(), SecretError> {
        self.erase(&Item::Credential(key.clone())).await
    }
}

/// Maps a keyring failure for `item` to what the UI can act on.
fn classify(item: &Item, error: keyring::Error) -> SecretError {
    if is_access_denied(&error) {
        return item.denied();
    }
    let unavailable = matches!(error, keyring::Error::NoStorageAccess(_));
    let source: PlatformError = Box::new(error);
    if unavailable {
        SecretError::Unavailable(source)
    } else {
        SecretError::Backend(source)
    }
}

/// OSStatus codes that mean "the item exists but you may not read it".
#[cfg(target_os = "macos")]
const ACCESS_DENIED_STATUSES: [i32; 3] = [
    -25293, // errSecAuthFailed: the item's ACL rejects this app's signature
    -128,   // errSecUserCanceled: the user clicked Deny on the Keychain prompt
    -25308, // errSecInteractionNotAllowed: no prompt is possible right now
];

/// True when macOS refused access to an existing item.
#[cfg(target_os = "macos")]
fn is_access_denied(error: &keyring::Error) -> bool {
    let (keyring::Error::PlatformFailure(source) | keyring::Error::NoStorageAccess(source)) = error
    else {
        return false;
    };
    source
        .downcast_ref::<security_framework::base::Error>()
        .is_some_and(|status| ACCESS_DENIED_STATUSES.contains(&status.code()))
}

/// Other platforms report a locked store as `NoStorageAccess` (unavailable).
#[cfg(not(target_os = "macos"))]
fn is_access_denied(_error: &keyring::Error) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: SecretName) -> Item {
        Item::Named(name)
    }

    #[test]
    fn a_missing_store_is_unavailable_not_a_generic_failure() {
        let error = keyring::Error::NoStorageAccess(Box::new(std::io::Error::other("locked")));

        let classified = classify(&named(SecretName::GroqApiKey), error);

        assert!(matches!(classified, SecretError::Unavailable(_)));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn an_acl_rejection_is_access_denied_for_that_secret() {
        let status = security_framework::base::Error::from_code(-25293);
        let error = keyring::Error::PlatformFailure(Box::new(status));

        let classified = classify(&named(SecretName::OpenAiApiKey), error);

        assert!(matches!(
            classified,
            SecretError::AccessDenied(SecretName::OpenAiApiKey)
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_denied_credential_asks_to_connect_again() {
        let status = security_framework::base::Error::from_code(-128);
        let error = keyring::Error::PlatformFailure(Box::new(status));
        let key = CredentialKey::parse("integration.slack.a").unwrap();

        let classified = classify(&Item::Credential(key), error);

        assert!(matches!(classified, SecretError::CredentialDenied));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn other_platform_failures_stay_internal() {
        let status = security_framework::base::Error::from_code(-25299);
        let error = keyring::Error::PlatformFailure(Box::new(status));

        assert!(matches!(
            classify(&named(SecretName::OpenAiApiKey), error),
            SecretError::Backend(_)
        ));
    }
}
