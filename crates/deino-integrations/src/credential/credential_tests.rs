use chrono::{TimeZone, Utc};
use deino_domain::FakeClock;
use deino_secrets::InMemoryCredentialStore;

use super::*;

fn secret(text: &str) -> SecretValue {
    SecretValue::parse(text).unwrap()
}

fn connection() -> ConnectionId {
    ConnectionId::generate(&FakeClock::new(
        Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap(),
    ))
}

#[tokio::test]
async fn a_credential_is_one_keychain_item_named_after_its_connection() {
    let store = Arc::new(InMemoryCredentialStore::default());
    let vault = CredentialVault::new(store.clone());
    let id = connection();
    let oauth = Credential::OAuth {
        access: secret("at-1"),
        refresh: Some(secret("rt-1")),
    };

    vault
        .save(IntegrationProvider::Linear, id, &oauth)
        .await
        .unwrap();
    let loaded = vault
        .load(IntegrationProvider::Linear, id)
        .await
        .unwrap()
        .unwrap();

    let keys = store.keys();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].as_str(), format!("integration.linear.{id}"));
    assert_eq!(loaded.refresh_token().unwrap().expose(), "rt-1");
    assert_eq!(loaded.revocable().expose(), "rt-1");

    vault.forget(IntegrationProvider::Linear, id).await.unwrap();
    assert!(
        vault
            .load(IntegrationProvider::Linear, id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn an_unreadable_item_counts_as_no_credential() {
    let store = Arc::new(InMemoryCredentialStore::default());
    let vault = CredentialVault::new(store.clone());
    let id = connection();
    let key = connection_key(IntegrationProvider::Slack, id).unwrap();
    store.put(&key, secret("not json")).await.unwrap();

    assert!(
        vault
            .load(IntegrationProvider::Slack, id)
            .await
            .unwrap()
            .is_none()
    );
}

#[test]
fn pasted_tokens_are_sent_the_way_their_provider_expects() {
    let token = |username: Option<&str>| Credential::Token {
        token: secret("tok"),
        username: username.map(str::to_owned),
    };
    let header = |credential: &Credential, scheme| {
        credential
            .authorization(scheme)
            .unwrap()
            .expose()
            .to_owned()
    };

    assert_eq!(header(&token(None), TokenScheme::Bearer), "Bearer tok");
    assert_eq!(header(&token(None), TokenScheme::Bare), "tok");
    let basic = header(&token(Some("ana@acme.com")), TokenScheme::Basic);
    let decoded = STANDARD.decode(basic.trim_start_matches("Basic ")).unwrap();
    assert_eq!(decoded, b"ana@acme.com:tok");
    let oauth = Credential::OAuth {
        access: secret("at"),
        refresh: None,
    };
    assert_eq!(header(&oauth, TokenScheme::Bare), "Bearer at");
}

#[tokio::test]
async fn client_secrets_are_saved_and_forgotten_per_provider() {
    let store = Arc::new(InMemoryCredentialStore::default());
    let vault = CredentialVault::new(store.clone());

    vault
        .set_client_secret(IntegrationProvider::Notion, Some(secret("secret_1")))
        .await
        .unwrap();
    let saved = vault
        .client_secret(IntegrationProvider::Notion)
        .await
        .unwrap();
    vault
        .set_client_secret(IntegrationProvider::Notion, None)
        .await
        .unwrap();

    assert_eq!(saved.unwrap().expose(), "secret_1");
    assert!(store.keys().is_empty());
}
