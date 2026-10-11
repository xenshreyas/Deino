//! [`ProviderApi`]: the authorized HTTP client a provider adapter is handed.
//!
//! The service builds one per call: a client confined to the provider's
//! `ServiceId` (so policy, offline mode and the egress log apply), the
//! connection's `Authorization` header, its per-account API base URL, and the
//! idempotency key of the delivery being sent. Adapters never see the
//! credential itself, and never classify transport policy errors: a non-2xx
//! answer comes back as [`ProviderError::Http`], which the provider's own
//! [`Integration::classify`](crate::Integration::classify) turns into
//! "unauthorized", "rate limited", "unavailable" or "rejected".

mod failure;

use std::fmt;
use std::sync::Arc;

use deino_net::{NetClient, Request, Response};
use deino_secrets::SecretValue;
use serde::de::DeserializeOwned;
use url::Url;

pub use failure::{Failure, MAX_FAILURE_BODY, standard_classification};

use crate::{IdempotencyKey, ProviderError};

/// The authorized client of one connection (see the module docs).
#[derive(Clone)]
pub struct ProviderApi {
    client: Arc<dyn NetClient>,
    authorization: Option<Arc<SecretValue>>,
    base_url: Option<Url>,
    idempotency_key: Option<IdempotencyKey>,
}

impl ProviderApi {
    /// A client that sends `authorization` as the `Authorization` header
    /// of every request (`None` before the user has connected).
    pub fn new(client: Arc<dyn NetClient>, authorization: Option<SecretValue>) -> Self {
        Self {
            client,
            authorization: authorization.map(Arc::new),
            base_url: None,
            idempotency_key: None,
        }
    }

    /// The same client for an account whose API lives at `base_url`.
    #[must_use]
    pub fn with_base_url(mut self, base_url: Option<Url>) -> Self {
        self.base_url = base_url;
        self
    }

    /// The same client while sending the delivery with `key`.
    #[must_use]
    pub fn with_idempotency_key(mut self, key: IdempotencyKey) -> Self {
        self.idempotency_key = Some(key);
        self
    }

    /// Where this account's API lives, when the provider decides it per
    /// account (Zoho, Salesforce, Jira).
    pub fn base_url(&self) -> Option<&Url> {
        self.base_url.as_ref()
    }

    /// The key of the delivery being sent, for providers that deduplicate
    /// writes themselves (an `Idempotency-Key` header, a client message ID).
    pub fn idempotency_key(&self) -> Option<&IdempotencyKey> {
        self.idempotency_key.as_ref()
    }

    /// Sends `request` with the connection's authorization. A 2xx answer is
    /// returned; anything else becomes [`ProviderError::Http`].
    pub async fn send(&self, request: Request) -> Result<Response, ProviderError> {
        let request = match &self.authorization {
            Some(value) => request.header("authorization", value.expose()),
            None => request,
        };
        let response = self.client.send(request).await?;
        if response.status.is_success() {
            return Ok(response);
        }
        Err(ProviderError::Http(Failure::read(response).await))
    }

    /// Sends `request` and decodes its 2xx JSON answer.
    pub async fn json<T: DeserializeOwned>(&self, request: Request) -> Result<T, ProviderError> {
        let request = request.header("accept", "application/json");
        Ok(self.send(request).await?.json().await?)
    }
}

impl fmt::Debug for ProviderApi {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderApi")
            .field("authorized", &self.authorization.is_some())
            .field("base_url", &self.base_url.as_ref().map(Url::as_str))
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use deino_net::{FakeNetClient, FakeReply, Status};

    use super::*;

    fn api(fake: &Arc<FakeNetClient>) -> ProviderApi {
        let token = SecretValue::parse("Bearer xoxp-1").unwrap();
        ProviderApi::new(fake.clone(), Some(token))
    }

    #[tokio::test]
    async fn every_request_carries_the_connection_authorization() {
        let fake = Arc::new(FakeNetClient::default());
        fake.reply(FakeReply::ok(r#"{ "ok": true }"#));

        let answer: serde_json::Value = api(&fake)
            .json(Request::get("https://slack.com/api/auth.test").unwrap())
            .await
            .unwrap();

        assert_eq!(answer["ok"], true);
        let sent = fake.requests();
        assert_eq!(sent[0].headers.get("authorization"), Some("Bearer xoxp-1"));
    }

    #[tokio::test]
    async fn a_non_success_answer_is_kept_for_the_provider_to_classify() {
        let fake = Arc::new(FakeNetClient::default());
        fake.reply(FakeReply::status(Status::new(429)).header("retry-after", "7"));

        let error = api(&fake)
            .send(Request::get("https://slack.com/api/auth.test").unwrap())
            .await
            .unwrap_err();

        let ProviderError::Http(failure) = error else {
            panic!("expected an HTTP failure, got {error:?}");
        };
        assert_eq!(failure.status, Status::new(429));
        assert_eq!(failure.header("retry-after"), Some("7"));
    }

    #[test]
    fn debug_output_never_shows_the_credential() {
        let fake = Arc::new(FakeNetClient::default());
        let printed = format!("{:?}", api(&fake));
        assert!(!printed.contains("xoxp"), "{printed}");
    }
}
