//! [`SecretStore`] over the OS keychain, through the `keyring` crate.
//!
//! Each secret is a generic password with the service set to the app's bundle
//! identifier (`com.xenshreyas.deino`) and the account set to
//! [`SecretName::account`]. Keychain calls block, sometimes on a system
//! prompt, so each runs on Tokio's blocking pool.

use async_trait::async_trait;

use crate::error::PlatformError;
use crate::{SecretError, SecretName, SecretStore, SecretValue};

/// Secrets in the macOS Keychain (the platform's native store elsewhere).
#[derive(Clone, Debug)]
pub struct KeychainSecretStore {
    service: String,
}

impl KeychainSecretStore {
    /// A store whose items carry `service`: the app's bundle identifier, so
    /// a build with another identifier never sees this app's keys.
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    /// Runs `operation` on the keychain entry for `name`, off the async runtime.
    async fn with_entry<T, F>(&self, name: SecretName, operation: F) -> Result<T, SecretError>
    where
        T: Send + 'static,
        F: FnOnce(&keyring::Entry) -> keyring::Result<T> + Send + 'static,
    {
        let service = self.service.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let entry = keyring::Entry::new(&service, name.account())?;
            operation(&entry)
        })
        .await
        .map_err(|join| SecretError::Backend(Box::new(join)))?;
        outcome.map_err(|error| classify(name, error))
    }
}

#[async_trait]
impl SecretStore for KeychainSecretStore {
    async fn set(&self, name: SecretName, value: SecretValue) -> Result<(), SecretError> {
        self.with_entry(name, move |entry| entry.set_password(value.expose()))
            .await
    }

    async fn get(&self, name: SecretName) -> Result<Option<SecretValue>, SecretError> {
        let stored = self
            .with_entry(name, |entry| match entry.get_password() {
                Ok(password) => Ok(Some(zeroize::Zeroizing::new(password))),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(error) => Err(error),
            })
            .await?;
        // A blank item (written by another tool) counts as no key at all.
        Ok(stored.and_then(|password| SecretValue::parse(&password).ok()))
    }

    async fn delete(&self, name: SecretName) -> Result<(), SecretError> {
        self.with_entry(name, |entry| match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error),
        })
        .await
    }
}

/// Maps a keyring failure for `name` to what the UI can act on.
fn classify(name: SecretName, error: keyring::Error) -> SecretError {
    if is_access_denied(&error) {
        return SecretError::AccessDenied(name);
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

    #[test]
    fn a_missing_store_is_unavailable_not_a_generic_failure() {
        let error = keyring::Error::NoStorageAccess(Box::new(std::io::Error::other("locked")));

        let classified = classify(SecretName::GroqApiKey, error);

        assert!(matches!(classified, SecretError::Unavailable(_)));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn an_acl_rejection_is_access_denied_for_that_secret() {
        let status = security_framework::base::Error::from_code(-25293);
        let error = keyring::Error::PlatformFailure(Box::new(status));

        let classified = classify(SecretName::OpenAiApiKey, error);

        assert!(matches!(
            classified,
            SecretError::AccessDenied(SecretName::OpenAiApiKey)
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn other_platform_failures_stay_internal() {
        let status = security_framework::base::Error::from_code(-25299);
        let error = keyring::Error::PlatformFailure(Box::new(status));

        assert!(matches!(
            classify(SecretName::OpenAiApiKey, error),
            SecretError::Backend(_)
        ));
    }
}
