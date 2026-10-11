//! [`InMemorySecretStore`] and [`InMemoryCredentialStore`]: the fakes for
//! tests, behind `test-support`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;

use crate::{CredentialKey, CredentialStore, SecretError, SecretName, SecretStore, SecretValue};

/// Keeps secrets in a map and can simulate a keychain that refuses access.
#[derive(Debug, Default)]
pub struct InMemorySecretStore {
    values: Mutex<BTreeMap<SecretName, String>>,
    denied: Mutex<BTreeSet<SecretName>>,
}

impl InMemorySecretStore {
    /// Makes reads of `name` fail with `AccessDenied` until it is set again,
    /// as the macOS Keychain does after an ad-hoc build's signature changes.
    pub fn deny(&self, name: SecretName) {
        lock(&self.denied).insert(name);
    }

    /// The raw saved value, for tests that assert what was stored.
    pub fn stored(&self, name: SecretName) -> Option<String> {
        lock(&self.values).get(&name).cloned()
    }

    /// Every name with a saved value.
    pub fn names(&self) -> Vec<SecretName> {
        lock(&self.values).keys().copied().collect()
    }
}

#[async_trait]
impl SecretStore for InMemorySecretStore {
    async fn set(&self, name: SecretName, value: SecretValue) -> Result<(), SecretError> {
        lock(&self.values).insert(name, value.expose().to_owned());
        lock(&self.denied).remove(&name);
        Ok(())
    }

    async fn get(&self, name: SecretName) -> Result<Option<SecretValue>, SecretError> {
        let Some(value) = lock(&self.values).get(&name).cloned() else {
            return Ok(None);
        };
        if lock(&self.denied).contains(&name) {
            return Err(SecretError::AccessDenied(name));
        }
        SecretValue::parse(&value).map(Some)
    }

    async fn delete(&self, name: SecretName) -> Result<(), SecretError> {
        lock(&self.values).remove(&name);
        lock(&self.denied).remove(&name);
        Ok(())
    }
}

/// Keeps credentials in a map and can simulate a keychain that refuses access.
#[derive(Debug, Default)]
pub struct InMemoryCredentialStore {
    values: Mutex<BTreeMap<CredentialKey, String>>,
    denied: Mutex<BTreeSet<CredentialKey>>,
}

impl InMemoryCredentialStore {
    /// Makes reads of `key` fail with `CredentialDenied` until it is put again.
    pub fn deny(&self, key: &CredentialKey) {
        lock(&self.denied).insert(key.clone());
    }

    /// The raw saved value, for tests that assert what was stored.
    pub fn stored(&self, key: &CredentialKey) -> Option<String> {
        lock(&self.values).get(key).cloned()
    }

    /// Every key with a saved value.
    pub fn keys(&self) -> Vec<CredentialKey> {
        lock(&self.values).keys().cloned().collect()
    }
}

#[async_trait]
impl CredentialStore for InMemoryCredentialStore {
    async fn put(&self, key: &CredentialKey, value: SecretValue) -> Result<(), SecretError> {
        lock(&self.values).insert(key.clone(), value.expose().to_owned());
        lock(&self.denied).remove(key);
        Ok(())
    }

    async fn read(&self, key: &CredentialKey) -> Result<Option<SecretValue>, SecretError> {
        let Some(value) = lock(&self.values).get(key).cloned() else {
            return Ok(None);
        };
        if lock(&self.denied).contains(key) {
            return Err(SecretError::CredentialDenied);
        }
        SecretValue::parse_credential(&value).map(Some)
    }

    async fn remove(&self, key: &CredentialKey) -> Result<(), SecretError> {
        lock(&self.values).remove(key);
        lock(&self.denied).remove(key);
        Ok(())
    }
}

/// A panic in another test thread cannot leave a map half-updated.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
