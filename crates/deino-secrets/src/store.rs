//! The `SecretStore` port.

use async_trait::async_trait;
use serde::Serialize;

use crate::{SecretError, SecretName, SecretValue};

/// Whether a secret is saved, as far as the UI may know.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SecretStatus {
    /// Nothing is saved under this name.
    Missing,
    /// A value is saved and readable.
    Stored,
    /// A value is saved, but the OS refused to hand it over (see
    /// [`SecretError::AccessDenied`]); the user should enter it again.
    Locked,
}

/// Where secrets live.
///
/// Implementations honour this contract, which `contract_tests.rs` checks
/// against each of them:
///
/// - `set` replaces any saved value; `get` then returns it;
/// - `delete` of a missing secret succeeds;
/// - `get` of a missing secret is `Ok(None)`;
/// - a secret the OS refuses to read fails with `AccessDenied`, and its
///   status is `Locked`.
///
/// `get` is for Rust consumers only (the LLM clients). The app exposes
/// `set`, `status` and `delete` to the UI, never `get`.
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Saves `value` under `name`, replacing any saved value.
    async fn set(&self, name: SecretName, value: SecretValue) -> Result<(), SecretError>;

    /// The saved value, or `None` when nothing is saved.
    async fn get(&self, name: SecretName) -> Result<Option<SecretValue>, SecretError>;

    /// Forgets the saved value; succeeds when nothing is saved.
    async fn delete(&self, name: SecretName) -> Result<(), SecretError>;

    /// Whether a value is saved and readable. Defaults to reading it.
    async fn status(&self, name: SecretName) -> Result<SecretStatus, SecretError> {
        match self.get(name).await {
            Ok(Some(_)) => Ok(SecretStatus::Stored),
            Ok(None) => Ok(SecretStatus::Missing),
            Err(SecretError::AccessDenied(_)) => Ok(SecretStatus::Locked),
            Err(error) => Err(error),
        }
    }
}
