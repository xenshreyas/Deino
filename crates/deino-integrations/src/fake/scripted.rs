//! [`ScriptedIntegration`]: a provider whose answers a test scripts.
//!
//! It speaks real HTTP through the [`ProviderApi`] it is given, so a test
//! scripts its answers with a `FakeNetClient` (or a local mock server) and
//! the whole path runs: the scoped client, policy, the egress log, the
//! Authorization header, classification, retries and refresh. Like Linear,
//! it signals rate limits as `400` with `RATELIMITED` in the body, so tests
//! also cover a provider-specific classifier.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::IntegrationProvider;
use deino_net::Request;
use serde::Deserialize;
use serde_json::json;

use crate::{
    Account, AuthSpec, CreateTask, ExternalRef, Failure, Integration, MessageDraft, PostMessage,
    ProviderApi, ProviderDescriptor, ProviderError, RateLimit, Target, TaskDraft, TokenExtras,
    standard_classification,
};

/// See the module docs.
#[derive(Debug)]
pub struct ScriptedIntegration {
    descriptor: ProviderDescriptor,
    api_base: String,
}

/// `GET /me`.
#[derive(Deserialize)]
struct Me {
    id: String,
    name: String,
}

impl ScriptedIntegration {
    /// A provider claiming to be `id`, connecting by `auth`, whose API is at
    /// `api_base` (`https://api.linear.app`, or a mock server).
    pub fn new(id: IntegrationProvider, auth: Vec<AuthSpec>, api_base: &str) -> Self {
        Self {
            descriptor: ProviderDescriptor {
                id,
                auth,
                rate_limit: RateLimit {
                    requests: 100,
                    per: Duration::from_secs(1),
                },
            },
            api_base: api_base.trim_end_matches('/').to_owned(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.api_base)
    }

    async fn create(
        &self,
        api: &ProviderApi,
        path: &str,
        body: serde_json::Value,
    ) -> Result<ExternalRef, ProviderError> {
        let mut request = Request::post(&self.url(path))?.json(&body)?;
        if let Some(key) = api.idempotency_key() {
            request = request.header("idempotency-key", key.as_str());
        }
        api.json(request).await
    }
}

#[async_trait]
impl Integration for ScriptedIntegration {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    async fn identify(
        &self,
        api: &ProviderApi,
        _extras: &TokenExtras,
    ) -> Result<Account, ProviderError> {
        let me: Me = api.json(Request::get(&self.url("me"))?).await?;
        Ok(Account {
            id: me.id,
            label: me.name,
            api_base_url: None,
        })
    }

    fn classify(&self, failure: &Failure, now: DateTime<Utc>) -> ProviderError {
        if failure.status.as_u16() == 400 && failure.body_contains("RATELIMITED") {
            return ProviderError::RateLimited { retry_after: None };
        }
        standard_classification(failure, now)
    }

    fn messages(&self) -> Option<&dyn PostMessage> {
        Some(self)
    }

    fn tasks(&self) -> Option<&dyn CreateTask> {
        Some(self)
    }
}

#[async_trait]
impl PostMessage for ScriptedIntegration {
    async fn message_targets(&self, api: &ProviderApi) -> Result<Vec<Target>, ProviderError> {
        api.json(Request::get(&self.url("channels"))?).await
    }

    async fn post_message(
        &self,
        api: &ProviderApi,
        draft: &MessageDraft,
    ) -> Result<ExternalRef, ProviderError> {
        let body = json!({ "channel": draft.target, "text": draft.text, "replies": draft.replies });
        self.create(api, "messages", body).await
    }
}

#[async_trait]
impl CreateTask for ScriptedIntegration {
    async fn task_targets(&self, api: &ProviderApi) -> Result<Vec<Target>, ProviderError> {
        api.json(Request::get(&self.url("teams"))?).await
    }

    async fn create_task(
        &self,
        api: &ProviderApi,
        draft: &TaskDraft,
    ) -> Result<ExternalRef, ProviderError> {
        let body = json!({ "team": draft.project, "title": draft.title });
        self.create(api, "tasks", body).await
    }
}
