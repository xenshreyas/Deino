//! What the Integrations page shows and sends: providers with their ways to
//! connect, the user's app registration, and their connections. Built from
//! the registry, the settings and the database; never a secret.

use deino_domain::{Capability, IntegrationProvider};
use serde::{Deserialize, Serialize};

use crate::catalog::planned_capabilities;
use crate::{AppRegistration, AuthSpec, Connection, Integration, OAuthSpec, PortRule, TokenSpec};

/// One provider on the Integrations page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    /// The provider.
    pub id: IntegrationProvider,
    /// Its name.
    pub name: String,
    /// False for a provider that is planned but not in this build yet: the
    /// page lists it as coming soon, with nothing to connect.
    pub available: bool,
    /// What it can do (or, when not available, will do).
    pub capabilities: Vec<Capability>,
    /// The ways to connect, the recommended one first.
    pub methods: Vec<ConnectMethod>,
    /// The user's own app registration, for OAuth.
    pub app: Option<AppView>,
    /// The connected accounts.
    pub connections: Vec<Connection>,
}

/// A way to connect, as the page explains it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(tag = "method", rename_all = "camelCase")]
pub enum ConnectMethod {
    /// Sign in in the browser.
    #[serde(rename = "oauth", rename_all = "camelCase")]
    OAuth {
        /// What Deino asks for, each explained.
        scopes: Vec<ScopeView>,
        /// The user's app needs a client secret too.
        needs_secret: bool,
        /// The redirect URI to register in the user's app.
        redirect_uri: String,
        /// Where the user registers the app.
        app_setup_url: String,
    },
    /// Paste a token.
    #[serde(rename = "token", rename_all = "camelCase")]
    Token {
        /// What the provider calls it.
        label: String,
        /// The second field, if the token needs one (Jira's email).
        username_label: Option<String>,
        /// Where to create it.
        help_url: String,
    },
}

/// A scope and what it allows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScopeView {
    /// The provider's name for it.
    pub id: String,
    /// What it lets Deino do.
    pub description: String,
}

/// The user's app registration, without its secret.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
    /// The client ID.
    pub client_id: String,
    /// The redirect port, when the user changed it.
    pub redirect_port: Option<u16>,
    /// A client secret is saved in the Keychain.
    pub has_secret: bool,
}

/// A pasted token, sent once from the page to the Keychain.
#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TokenInput {
    /// The token or key.
    pub token: String,
    /// The second field, for providers that need one.
    pub username: Option<String>,
}

/// The user's app, as entered on the page.
#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInput {
    /// The client ID.
    pub client_id: String,
    /// A new client secret; `None` keeps the saved one.
    pub client_secret: Option<String>,
    /// The redirect port the app registered; `None` for the default.
    pub redirect_port: Option<u16>,
}

/// The view of `integration`, with the user's `app` and `connections`.
pub(crate) fn provider_view(
    integration: &dyn Integration,
    app: Option<(AppRegistration, bool)>,
    connections: Vec<Connection>,
) -> ProviderView {
    let port = app
        .as_ref()
        .and_then(|(registration, _)| registration.redirect_port);
    let descriptor = integration.descriptor();
    ProviderView {
        id: descriptor.id,
        name: descriptor.id.name().to_owned(),
        available: true,
        capabilities: integration.capabilities(),
        methods: descriptor
            .auth
            .iter()
            .map(|spec| method(spec, port))
            .collect(),
        app: app.map(|(registration, has_secret)| AppView {
            client_id: registration.client_id,
            redirect_port: registration.redirect_port,
            has_secret,
        }),
        connections,
    }
}

/// The view of a provider that is planned but not in this build.
pub(crate) fn planned_view(provider: IntegrationProvider) -> ProviderView {
    ProviderView {
        id: provider,
        name: provider.name().to_owned(),
        available: false,
        capabilities: planned_capabilities(provider).to_vec(),
        methods: Vec::new(),
        app: None,
        connections: Vec::new(),
    }
}

fn method(spec: &AuthSpec, port: Option<u16>) -> ConnectMethod {
    match spec {
        AuthSpec::OAuth(oauth) => oauth_method(oauth, port),
        AuthSpec::Token(token) => token_method(token),
    }
}

fn oauth_method(spec: &OAuthSpec, port: Option<u16>) -> ConnectMethod {
    ConnectMethod::OAuth {
        scopes: spec
            .scopes
            .iter()
            .map(|scope| ScopeView {
                id: scope.id.clone(),
                description: scope.description.clone(),
            })
            .collect(),
        needs_secret: spec.client_auth.needs_secret(),
        redirect_uri: redirect_uri(spec, port),
        app_setup_url: spec.app_setup_url.clone(),
    }
}

/// The redirect URI the user registers: an exact port for providers that
/// match it, the bare host for those that accept any port.
pub(crate) fn redirect_uri(spec: &OAuthSpec, port: Option<u16>) -> String {
    let host = spec.redirect.host.as_str();
    match spec.redirect.port {
        PortRule::Any => format!("http://{host}"),
        PortRule::Fixed(default) => {
            format!(
                "http://{host}:{}{}",
                port.unwrap_or(default),
                spec.redirect.path
            )
        }
    }
}

fn token_method(spec: &TokenSpec) -> ConnectMethod {
    ConnectMethod::Token {
        label: spec.label.to_owned(),
        username_label: spec.username_label.map(str::to_owned),
        help_url: spec.help_url.clone(),
    }
}
