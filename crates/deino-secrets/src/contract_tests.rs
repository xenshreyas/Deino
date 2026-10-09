//! The `SecretStore` contract, run against every implementation (§3A Liskov).
//!
//! The in-memory store runs in every `cargo test`. The keychain store touches
//! the login keychain, so its run is `#[ignore]`d; run it on a Mac with
//! `cargo test -p deino-secrets -- --ignored`. It uses its own service name,
//! so it never sees the app's real keys.

use crate::{
    InMemorySecretStore, KeychainSecretStore, SecretName, SecretStatus, SecretStore, SecretValue,
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
