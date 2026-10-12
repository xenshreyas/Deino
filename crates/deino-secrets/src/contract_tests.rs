//! The `SecretStore` contract, run against every implementation (§3A Liskov).
//!
//! The in-memory store runs in every `cargo test`. The keychain store touches
//! the login keychain, so its run is `#[ignore]`d; run it on a Mac with
//! `cargo test -p deino-secrets -- --ignored`. It uses its own service name,
//! so it never sees the app's real keys.

use crate::{
    CredentialKey, CredentialStore, DatabaseKeyStore, InMemoryCredentialStore,
    InMemoryDatabaseKeyStore, InMemorySecretStore, KeychainSecretStore, MAX_SECRET_CHARS,
    SecretError, SecretName, SecretStatus, SecretStore, SecretValue,
};

/// Checks every rule of the `SecretStore` contract except access denial,
/// which only the fake can trigger on demand.
async fn honours_the_contract(store: &dyn SecretStore) {
    let name = SecretName::GroqApiKey;
    store.delete(name).await.unwrap();
    a_missing_secret_reads_as_none(store, name).await;

    store.set(name, value("gsk-first")).await.unwrap();
    store.set(name, value("gsk-second")).await.unwrap();
    let saved = store.get(name).await.unwrap().unwrap();
    assert_eq!(saved.expose(), "gsk-second");
    assert_eq!(store.status(name).await.unwrap(), SecretStatus::Stored);

    store.delete(name).await.unwrap();
    store.delete(name).await.unwrap();
    a_missing_secret_reads_as_none(store, name).await;
}

async fn a_missing_secret_reads_as_none(store: &dyn SecretStore, name: SecretName) {
    assert!(store.get(name).await.unwrap().is_none());
    assert_eq!(store.status(name).await.unwrap(), SecretStatus::Missing);
}

fn value(text: &str) -> SecretValue {
    SecretValue::parse(text).unwrap()
}

#[tokio::test]
async fn the_in_memory_store_honours_the_contract() {
    honours_the_contract(&InMemorySecretStore::default()).await;
}

#[tokio::test]
#[ignore = "reads and writes the login keychain; run by hand on a Mac"]
async fn the_keychain_store_honours_the_contract() {
    let store = KeychainSecretStore::new("com.xenshreyas.deino.contract-tests");
    honours_the_contract(&store).await;
}

/// The `CredentialStore` contract: the same rules, over run-time keys and
/// values longer than an API key.
async fn credentials_honour_the_contract(store: &dyn CredentialStore) {
    let key = CredentialKey::parse("integration.sandbox.contract-test").unwrap();
    store.remove(&key).await.unwrap();
    assert!(store.read(&key).await.unwrap().is_none());

    let long = "t".repeat(MAX_SECRET_CHARS * 2);
    store.put(&key, credential("first")).await.unwrap();
    store.put(&key, credential(&long)).await.unwrap();
    assert_eq!(store.read(&key).await.unwrap().unwrap().expose(), long);

    store.remove(&key).await.unwrap();
    store.remove(&key).await.unwrap();
    assert!(store.read(&key).await.unwrap().is_none());
}

fn credential(text: &str) -> SecretValue {
    SecretValue::parse_credential(text).unwrap()
}

#[tokio::test]
async fn the_in_memory_credential_store_honours_the_contract() {
    credentials_honour_the_contract(&InMemoryCredentialStore::default()).await;
}

#[tokio::test]
#[ignore = "reads and writes the login keychain; run by hand on a Mac"]
async fn the_keychain_credential_store_honours_the_contract() {
    let store = KeychainSecretStore::new("com.xenshreyas.deino.contract-tests");
    credentials_honour_the_contract(&store).await;
}

#[tokio::test]
async fn a_denied_credential_fails_until_it_is_saved_again() {
    let store = InMemoryCredentialStore::default();
    let key = CredentialKey::parse("integration.slack.a").unwrap();
    store.put(&key, credential("xoxp-1")).await.unwrap();

    store.deny(&key);

    assert!(matches!(
        store.read(&key).await,
        Err(SecretError::CredentialDenied)
    ));
    store.put(&key, credential("xoxp-2")).await.unwrap();
    assert_eq!(store.read(&key).await.unwrap().unwrap().expose(), "xoxp-2");
}

#[tokio::test]
async fn a_denied_secret_is_locked_until_it_is_entered_again() {
    let store = InMemorySecretStore::default();
    let name = SecretName::OpenAiApiKey;
    store.set(name, value("sk-1")).await.unwrap();

    store.deny(name);

    assert_eq!(store.status(name).await.unwrap(), SecretStatus::Locked);
    assert!(store.get(name).await.is_err());
    store.set(name, value("sk-2")).await.unwrap();
    assert_eq!(store.status(name).await.unwrap(), SecretStatus::Stored);
}

/// The `DatabaseKeyStore` contract (PRV-02): save replaces, a missing key
/// reads as `None`, forget is idempotent.
async fn database_keys_honour_the_contract(store: &dyn DatabaseKeyStore) {
    store.forget_database_key().await.unwrap();
    assert!(store.load_database_key().await.unwrap().is_none());
    store.forget_database_key().await.unwrap();

    let first = "0f".repeat(32);
    let second = "a1".repeat(32);
    store.save_database_key(value(&first)).await.unwrap();
    store.save_database_key(value(&second)).await.unwrap();
    let loaded = store.load_database_key().await.unwrap().unwrap();
    assert_eq!(loaded.expose(), second);

    store.forget_database_key().await.unwrap();
    assert!(store.load_database_key().await.unwrap().is_none());
}

#[tokio::test]
async fn the_in_memory_database_key_store_honours_the_contract() {
    database_keys_honour_the_contract(&InMemoryDatabaseKeyStore::default()).await;
}

#[tokio::test]
#[ignore = "reads and writes the login keychain; run by hand on a Mac"]
async fn the_keychain_database_key_store_honours_the_contract() {
    // A service of its own: never the app's real database key.
    let store = KeychainSecretStore::new("dev.deino.prv02.contract-tests");
    database_keys_honour_the_contract(&store).await;
}

#[tokio::test]
async fn a_denied_database_key_fails_until_it_is_saved_again() {
    let store = InMemoryDatabaseKeyStore::default();
    store
        .save_database_key(value(&"0f".repeat(32)))
        .await
        .unwrap();

    store.deny();

    let denied = store.load_database_key().await.unwrap_err();
    assert!(
        matches!(denied, SecretError::DatabaseKeyDenied),
        "{denied:?}"
    );
    store
        .save_database_key(value(&"a1".repeat(32)))
        .await
        .unwrap();
    assert!(store.load_database_key().await.unwrap().is_some());
}
