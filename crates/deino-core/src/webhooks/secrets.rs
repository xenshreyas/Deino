//! [`WebhookSecrets`]: each endpoint's signing secret in the Keychain, one
//! item per endpoint (`integration.webhook.<id>`), through the same
//! [`CredentialStore`] port as integration tokens (ADR 0031). The secret is
//! never stored in SQLite, logged, or sent to the UI after it is created.

use std::sync::Arc;

use deino_domain::WebhookId;
use deino_secrets::{CredentialKey, CredentialStore, SecretValue};

use super::{SigningSecret, WebhookError};

/// The signing secrets, by endpoint.
#[derive(Clone)]
pub struct WebhookSecrets {
    store: Arc<dyn CredentialStore>,
}

impl WebhookSecrets {
    /// The secrets over `store` (the Keychain in the app).
    pub fn new(store: Arc<dyn CredentialStore>) -> Self {
        Self { store }
    }

    /// Saves `secret` for `webhook`.
    pub async fn save(
        &self,
        webhook: &WebhookId,
        secret: &SigningSecret,
    ) -> Result<(), WebhookError> {
        let value = SecretValue::parse(secret.expose())?;
        self.store.put(&key(webhook)?, value).await?;
        Ok(())
    }

    /// `webhook`'s secret.
    pub async fn load(&self, webhook: &WebhookId) -> Result<SigningSecret, WebhookError> {
        let value = self
            .store
            .read(&key(webhook)?)
            .await?
            .ok_or(WebhookError::BadSecret)?;
        SigningSecret::parse(value.expose())
    }

    /// Forgets `webhook`'s secret.
    pub async fn forget(&self, webhook: &WebhookId) -> Result<(), WebhookError> {
        self.store.remove(&key(webhook)?).await?;
        Ok(())
    }
}

/// The Keychain account of `webhook`'s secret.
fn key(webhook: &WebhookId) -> Result<CredentialKey, WebhookError> {
    Ok(CredentialKey::parse(&format!(
        "integration.webhook.{webhook}"
    ))?)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use deino_domain::FakeClock;
    use deino_secrets::InMemoryCredentialStore;

    use super::*;

    #[tokio::test]
    async fn a_secret_round_trips_through_the_store_and_can_be_forgotten() {
        let store = Arc::new(InMemoryCredentialStore::default());
        let secrets = WebhookSecrets::new(store);
        let start = Utc.with_ymd_and_hms(2026, 10, 12, 9, 0, 0).unwrap();
        let id = WebhookId::generate(&FakeClock::new(start));
        let secret = SigningSecret::generate().unwrap();

        secrets.save(&id, &secret).await.unwrap();
        let loaded = secrets.load(&id).await.unwrap();
        secrets.forget(&id).await.unwrap();

        assert_eq!(loaded.expose(), secret.expose());
        assert!(matches!(
            secrets.load(&id).await,
            Err(WebhookError::BadSecret)
        ));
    }
}
