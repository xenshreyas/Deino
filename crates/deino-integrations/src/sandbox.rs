//! [`SandboxIntegration`]: a provider that talks to nothing.
//!
//! Debug builds register it so the whole connect, deliver and disconnect
//! path can be exercised in the dev app and in `pnpm dev:mock` without an
//! account anywhere; release builds never list it. It connects with any
//! pasted token, offers every capability, and "creates" items that exist
//! only as their returned IDs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use deino_domain::IntegrationProvider;

use crate::{
    Account, ActivityDraft, AuthSpec, ContextItem, ContextQuery, CreatePage, CreateTask,
    ExternalRef, Integration, LogActivity, MessageDraft, PageDraft, PostMessage, ProviderApi,
    ProviderDescriptor, ProviderError, RateLimit, ReadContext, Target, TaskDraft, TokenExtras,
    TokenScheme, TokenSpec,
};

/// The sandbox's one account.
const ACCOUNT_ID: &str = "sandbox";

/// See the module docs.
#[derive(Debug)]
pub struct SandboxIntegration {
    descriptor: ProviderDescriptor,
    created: AtomicU64,
}

impl Default for SandboxIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxIntegration {
    /// The sandbox provider.
    pub fn new() -> Self {
        let token = TokenSpec {
            label: "Sandbox token",
            username_label: None,
            scheme: TokenScheme::Bearer,
            help_url: "https://example.invalid/sandbox".into(),
        };
        Self {
            descriptor: ProviderDescriptor {
                id: IntegrationProvider::Sandbox,
                auth: vec![AuthSpec::Token(token)],
                rate_limit: RateLimit {
                    requests: 10,
                    per: Duration::from_secs(1),
                },
            },
            created: AtomicU64::new(0),
        }
    }

    fn created(&self, kind: &str) -> ExternalRef {
        let number = self.created.fetch_add(1, Ordering::Relaxed) + 1;
        ExternalRef {
            id: format!("sandbox-{kind}-{number}"),
            url: None,
        }
    }

    fn targets(names: &[&str]) -> Vec<Target> {
        names
            .iter()
            .map(|name| Target {
                id: name.to_lowercase().replace(' ', "-"),
                name: (*name).to_owned(),
            })
            .collect()
    }
}

#[async_trait]
impl Integration for SandboxIntegration {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    async fn identify(
        &self,
        _api: &ProviderApi,
        _extras: &TokenExtras,
    ) -> Result<Account, ProviderError> {
        Ok(Account {
            id: ACCOUNT_ID.into(),
            label: "Sandbox workspace".into(),
            api_base_url: None,
        })
    }

    fn messages(&self) -> Option<&dyn PostMessage> {
        Some(self)
    }

    fn pages(&self) -> Option<&dyn CreatePage> {
        Some(self)
    }

    fn tasks(&self) -> Option<&dyn CreateTask> {
        Some(self)
    }

    fn activities(&self) -> Option<&dyn LogActivity> {
        Some(self)
    }

    fn context(&self) -> Option<&dyn ReadContext> {
        Some(self)
    }
}

#[async_trait]
impl PostMessage for SandboxIntegration {
    async fn message_targets(&self, _api: &ProviderApi) -> Result<Vec<Target>, ProviderError> {
        Ok(Self::targets(&["#general", "#meeting-notes"]))
    }

    async fn post_message(
        &self,
        _api: &ProviderApi,
        _draft: &MessageDraft,
    ) -> Result<ExternalRef, ProviderError> {
        Ok(self.created("message"))
    }
}

#[async_trait]
impl CreatePage for SandboxIntegration {
    async fn page_targets(&self, _api: &ProviderApi) -> Result<Vec<Target>, ProviderError> {
        Ok(Self::targets(&["Meeting notes", "Team wiki"]))
    }

    async fn create_page(
        &self,
        _api: &ProviderApi,
        _draft: &PageDraft,
    ) -> Result<ExternalRef, ProviderError> {
        Ok(self.created("page"))
    }
}

#[async_trait]
impl CreateTask for SandboxIntegration {
    async fn task_targets(&self, _api: &ProviderApi) -> Result<Vec<Target>, ProviderError> {
        Ok(Self::targets(&["Inbox", "Product"]))
    }

    async fn create_task(
        &self,
        _api: &ProviderApi,
        _draft: &TaskDraft,
    ) -> Result<ExternalRef, ProviderError> {
        Ok(self.created("task"))
    }
}

#[async_trait]
impl LogActivity for SandboxIntegration {
    async fn log_activity(
        &self,
        _api: &ProviderApi,
        _draft: &ActivityDraft,
    ) -> Result<ExternalRef, ProviderError> {
        Ok(self.created("activity"))
    }
}

#[async_trait]
impl ReadContext for SandboxIntegration {
    async fn read_context(
        &self,
        _api: &ProviderApi,
        _query: &ContextQuery,
    ) -> Result<Vec<ContextItem>, ProviderError> {
        Ok(Vec::new())
    }
}
