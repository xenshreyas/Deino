//! [`Integration`]: what every provider implements, and the
//! [`ProviderDescriptor`] that says how it connects.
//!
//! A provider card (INT-01..07) adds one module that implements
//! [`Integration`] plus the capability traits it offers, and one line in
//! [`crate::registry`]. The framework does the rest: OAuth or token paste,
//! the Keychain, refresh, policy, rate limits, retries, idempotency and the
//! delivery log.

mod auth;

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deino_domain::{Capability, IntegrationProvider};
use url::Url;

pub use auth::{
    AuthSpec, ClientAuth, LoopbackHost, OAuthSpec, PortRule, RedirectSpec, Scope, TokenScheme,
    TokenSpec,
};

use crate::{
    CreatePage, CreateTask, Failure, LogActivity, PostMessage, ProviderApi, ProviderError,
    ReadContext, standard_classification,
};

/// How a provider connects and how hard it may be called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderDescriptor {
    /// Which provider this is.
    pub id: IntegrationProvider,
    /// The ways to connect, the recommended one first.
    pub auth: Vec<AuthSpec>,
    /// The pace Deino keeps below the provider's documented limit.
    pub rate_limit: RateLimit,
}

impl ProviderDescriptor {
    /// The provider's OAuth settings, if it connects through OAuth.
    pub fn oauth(&self) -> Option<&OAuthSpec> {
        self.auth.iter().find_map(|spec| match spec {
            AuthSpec::OAuth(oauth) => Some(oauth),
            AuthSpec::Token(_) => None,
        })
    }

    /// The provider's token paste settings, if it accepts a pasted token.
    pub fn token(&self) -> Option<&TokenSpec> {
        self.auth.iter().find_map(|spec| match spec {
            AuthSpec::Token(token) => Some(token),
            AuthSpec::OAuth(_) => None,
        })
    }
}

/// At most `requests` calls per `per`, per connection (a token bucket that
/// allows bursts of `requests`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateLimit {
    /// Calls allowed in each window, and the largest burst.
    pub requests: u32,
    /// The window.
    pub per: Duration,
}

/// The account a credential belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    /// The provider's stable ID for the account or workspace.
    pub id: String,
    /// What the user recognises: `Acme · ana@acme.com`.
    pub label: String,
    /// Where this account's API lives, when the provider decides it per
    /// account (Zoho's `api_domain`, Salesforce's `instance_url`).
    pub api_base_url: Option<Url>,
}

/// Top-level string fields of a token response beyond the standard ones
/// (`api_domain`, `instance_url`, `workspace_name`), for
/// [`Integration::identify`]. Empty for pasted tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenExtras(BTreeMap<String, String>);

impl TokenExtras {
    /// Extras from name-value pairs.
    pub fn new(fields: BTreeMap<String, String>) -> Self {
        Self(fields)
    }

    /// The value of the field `name`, if the provider sent one.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }
}

/// A provider Deino can connect to (see the module docs).
///
/// Contract, checked for every provider by `provider_contract_tests.rs`:
/// `identify` names the same account for the same credential;
/// `capabilities` lists exactly the accessors that return `Some`; adapters
/// return non-2xx answers as [`ProviderError::Http`] (through
/// [`ProviderApi::send`]) and leave their meaning to [`Integration::classify`].
#[async_trait]
pub trait Integration: Send + Sync {
    /// How the provider connects and how hard it may be called.
    fn descriptor(&self) -> &ProviderDescriptor;

    /// The account a fresh credential belongs to.
    async fn identify(
        &self,
        api: &ProviderApi,
        extras: &TokenExtras,
    ) -> Result<Account, ProviderError>;

    /// What a non-2xx answer means. The default is
    /// [`standard_classification`]; providers that signal limits their own
    /// way (Linear's `400 RATELIMITED`, Salesforce's `403
    /// REQUEST_LIMIT_EXCEEDED`) override it.
    fn classify(&self, failure: &Failure, now: DateTime<Utc>) -> ProviderError {
        standard_classification(failure, now)
    }

    /// Revokes the credential at the provider when it has a non-standard
    /// revocation call (Slack's `auth.revoke`). The framework already calls
    /// an RFC 7009 `revoke_url`; the default does nothing more.
    async fn revoke(&self, _api: &ProviderApi) -> Result<(), ProviderError> {
        Ok(())
    }

    /// Posting messages, if the provider can.
    fn messages(&self) -> Option<&dyn PostMessage> {
        None
    }

    /// Creating pages, if the provider can.
    fn pages(&self) -> Option<&dyn CreatePage> {
        None
    }

    /// Creating tasks, if the provider can.
    fn tasks(&self) -> Option<&dyn CreateTask> {
        None
    }

    /// Logging CRM activities, if the provider can.
    fn activities(&self) -> Option<&dyn LogActivity> {
        None
    }

    /// Reading context, if the provider can.
    fn context(&self) -> Option<&dyn ReadContext> {
        None
    }

    /// Which provider this is.
    fn id(&self) -> IntegrationProvider {
        self.descriptor().id
    }

    /// Everything the provider can do, from its accessors.
    fn capabilities(&self) -> Vec<Capability> {
        let offered = [
            (Capability::PostMessage, self.messages().is_some()),
            (Capability::CreatePage, self.pages().is_some()),
            (Capability::CreateTask, self.tasks().is_some()),
            (Capability::LogActivity, self.activities().is_some()),
            (Capability::ReadContext, self.context().is_some()),
        ];
        offered
            .into_iter()
            .filter_map(|(capability, offered)| offered.then_some(capability))
            .collect()
    }
}
