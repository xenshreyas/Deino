//! Deino's secrets: API keys and tokens, kept in the OS keychain.
//!
//! Secrets never touch SQLite, JSON files, logs or the UI (PARITY_PLAN §3
//! rule 5). This crate owns the one port every consumer goes through:
//!
//! - [`SecretName`]: the closed set of secrets Deino knows, each stored under
//!   its own keychain account;
//! - [`SecretValue`]: a validated value that cannot be printed, serialized or
//!   left behind in memory;
//! - [`SecretStore`]: set, status, delete and get. The UI reaches the first
//!   three through the app's `secrets` plugin; [`SecretStore::get`] is for
//!   Rust consumers only (the LLM clients) and has no IPC command;
//! - [`KeychainSecretStore`]: the real store, over the `keyring` crate;
//! - `InMemorySecretStore` (feature `test-support`): the fake for tests;
//! - [`CredentialKey`] and [`CredentialStore`]: per-account integration
//!   credentials (INT-00), whose keychain account is decided at run time.
//!   [`KeychainSecretStore`] implements both ports, and
//!   `InMemoryCredentialStore` (feature `test-support`) is its fake.
//! - [`DatabaseKeyStore`]: the database encryption key (PRV-02), one item
//!   under [`DATABASE_KEY_ACCOUNT`]; `InMemoryDatabaseKeyStore` is its fake.
//!
//! A [`SecretStatus::Locked`] secret exists but the OS refused to hand it
//! over, which happens to ad-hoc-signed builds after the signature changes.
//! The UI shows it as "re-enter this key" instead of failing later.

mod credential;
mod database_key;
mod error;
mod keychain;
#[cfg(any(test, feature = "test-support"))]
mod memory;
mod name;
mod store;
mod value;

pub use credential::{CredentialKey, CredentialStore, MAX_CREDENTIAL_CHARS};
pub use database_key::{DATABASE_KEY_ACCOUNT, DatabaseKeyStore};
pub use error::SecretError;
pub use keychain::KeychainSecretStore;
#[cfg(any(test, feature = "test-support"))]
pub use memory::{InMemoryCredentialStore, InMemoryDatabaseKeyStore, InMemorySecretStore};
pub use name::SecretName;
pub use store::{SecretStatus, SecretStore};
pub use value::{MAX_SECRET_CHARS, SecretValue};

#[cfg(test)]
mod contract_tests;
