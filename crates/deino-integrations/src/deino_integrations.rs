//! Deino's integrations framework (INT-00, ADR 0029).
//!
//! Integrations are the one place data leaves the Mac for a service other
//! than the chosen LLM: always directly from this Mac to the provider's API,
//! with the user's own sign-in, never through a Deino server.
//!
//! - **Providers** implement [`Integration`] (a [`ProviderDescriptor`] and
//!   `identify`) plus the capability traits they offer ([`PostMessage`],
//!   [`CreatePage`], [`CreateTask`], [`LogActivity`], [`ReadContext`]).
//!   [`registry::integrations`] is the one list. [`SandboxIntegration`] talks
//!   to nothing (debug builds); `ScriptedIntegration` (feature
//!   `test-support`) answers what a test scripts.
//! - **Connecting** ([`oauth`]): OAuth 2.0 with PKCE through a loopback
//!   redirect on `127.0.0.1` (RFC 8252), per-provider redirect rules (any
//!   port, port-agnostic `localhost`, or the exact port of the user's app),
//!   refresh, RFC 7009 revocation and a device-code helper; or a pasted token
//!   ([`TokenScheme`]: `Bearer`, bare, or Basic with a username).
//! - **Credentials** ([`CredentialVault`]): one Keychain item per
//!   connection; never in SQLite, settings, logs or the webview.
//! - **Connections** ([`Connection`], `integration_connections`): provider,
//!   account, scopes, status, last use and last error.
//! - **Deliveries** ([`IntegrationService::deliver`], `integration_deliveries`):
//!   one write per [`IdempotencyKey`], paced by a per-connection
//!   [`RateLimiter`], retried with backoff, refreshed once on a 401, and
//!   logged for the Integrations page. Each provider classifies its own
//!   failures ([`Integration::classify`]).
//! - **Policy**: every request goes through `deino-net` scoped to the
//!   provider's `ServiceId` ([`Egress`]); connecting switches the service on,
//!   the last disconnect switches it off, and offline mode stops everything
//!   with an error that says so.

mod api;
mod capability;
mod catalog;
mod connection;
mod credential;
mod delivery;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
pub mod oauth;
mod ports;
mod provider;
#[cfg(test)]
mod provider_contract_tests;
mod rate_limit;
pub mod registry;
mod sandbox;
mod service;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;
mod view;

pub use api::{Failure, MAX_FAILURE_BODY, ProviderApi, standard_classification};
pub use capability::{
    Action, ActivityDraft, ContextItem, ContextQuery, CreatePage, CreateTask, LogActivity,
    MessageDraft, PageDraft, PostMessage, ReadContext, Target, TaskDraft,
};
pub use connection::{
    Connection, ConnectionError, ConnectionRepo, MAX_ERROR_CHARS, SqliteConnectionRepo,
};
pub use credential::{Credential, CredentialVault};
pub use delivery::{
    Delivery, DeliveryFailure, DeliveryRepo, DeliveryRequest, ExternalRef, IdempotencyKey,
    MAX_KEY_CHARS, NewDelivery, SqliteDeliveryRepo,
};
pub use error::{IntegrationError, ProviderError};
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeBrowser, FakeEgress, FakeOAuthApps, FakeSleeper, ScriptedIntegration};
pub use oauth::{
    AuthorizationFlow, IssuedTokens, LoopbackListener, OAuthClient, OAuthError, Pkce, TokenEndpoint,
};
pub use ports::{AppRegistration, Browser, ClockSleeper, Egress, NetEgress, OAuthApps, Sleeper};
pub use provider::{
    Account, AuthSpec, ClientAuth, Integration, LoopbackHost, OAuthSpec, PortRule,
    ProviderDescriptor, RateLimit, RedirectSpec, Scope, TokenExtras, TokenScheme, TokenSpec,
};
pub use rate_limit::RateLimiter;
pub use registry::{IntegrationRegistry, RegistryOptions};
pub use sandbox::SandboxIntegration;
pub use service::{IntegrationPorts, IntegrationService, MAX_DELIVERY_PAGE};
pub use view::{AppInput, AppView, ConnectMethod, ProviderView, ScopeView, TokenInput};
