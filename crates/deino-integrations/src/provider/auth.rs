//! How a provider connects: OAuth with PKCE over a loopback redirect, or a
//! pasted token. The per-provider constraints behind these types are in
//! `docs/research/integrations-oauth.md` (ADR 0029).

use deino_domain::AuthMethod;

/// One way to connect a provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthSpec {
    /// OAuth 2.0 authorization code with PKCE (RFC 7636) through a loopback
    /// redirect (RFC 8252 §7.3).
    OAuth(OAuthSpec),
    /// A token, API key or personal access token the user pastes.
    Token(TokenSpec),
}

impl AuthSpec {
    /// The stored method.
    pub const fn method(&self) -> AuthMethod {
        match self {
            Self::OAuth(_) => AuthMethod::OAuth,
            Self::Token(_) => AuthMethod::Token,
        }
    }
}

/// A provider's OAuth endpoints and rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuthSpec {
    /// Where the browser goes to approve access.
    pub authorize_url: String,
    /// Where the code is exchanged for tokens, and tokens are refreshed.
    pub token_url: String,
    /// RFC 7009 revocation, if the provider has one.
    pub revoke_url: Option<String>,
    /// The scopes Deino asks for, each explained to the user.
    pub scopes: Vec<Scope>,
    /// The query parameter that carries them: `scope`, or Slack's
    /// `user_scope` for desktop (user-token) apps.
    pub scope_param: &'static str,
    /// How scopes are joined: `' '` (RFC 6749) or `','` (Slack).
    pub scope_separator: char,
    /// Extra authorization parameters (`access_type=offline`, `owner=user`).
    pub extra_params: Vec<(&'static str, &'static str)>,
    /// Where the browser is sent back to.
    pub redirect: RedirectSpec,
    /// Whether, and how, the client secret is sent to the token endpoint.
    pub client_auth: ClientAuth,
    /// Where the user registers their own app (shown with the client ID field).
    pub app_setup_url: String,
}

/// A scope Deino asks for, as the Integrations page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    /// The provider's name for it: `chat:write`.
    pub id: String,
    /// What it lets Deino do, in plain words.
    pub description: String,
}

impl Scope {
    /// A scope from its ID and description.
    pub fn new(id: &str, description: &str) -> Self {
        Self {
            id: id.to_owned(),
            description: description.to_owned(),
        }
    }
}

/// How the client authenticates at the token endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientAuth {
    /// A public client: PKCE only, no secret (Slack with PKCE, Linear,
    /// Microsoft).
    Public,
    /// `client_secret` in the form body (Google's non-confidential Desktop
    /// secret, a user's own app).
    SecretInBody,
    /// HTTP Basic `client_id:client_secret` (Notion).
    SecretInBasicHeader,
}

impl ClientAuth {
    /// True when connecting needs a client secret.
    pub const fn needs_secret(self) -> bool {
        !matches!(self, Self::Public)
    }
}

/// The loopback address the browser is sent back to. No provider accepts
/// every form (`docs/research/integrations-oauth.md`): Google wants
/// `127.0.0.1` on any port, Microsoft `localhost` with the port ignored, and
/// Slack and Linear the exact `localhost` port the user's app registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RedirectSpec {
    /// The host in the redirect URI.
    pub host: LoopbackHost,
    /// Which port the listener binds.
    pub port: PortRule,
    /// The path, starting with `/`.
    pub path: &'static str,
}

/// The host part of a loopback redirect. The listener always binds
/// `127.0.0.1`; `localhost` also binds `[::1]` when it can, since browsers
/// may resolve it to either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopbackHost {
    /// `127.0.0.1` (RFC 8252 §7.3's recommendation; Google).
    Ipv4,
    /// `localhost` (Microsoft, Slack, Linear).
    Localhost,
}

impl LoopbackHost {
    /// The host as written in the redirect URI.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ipv4 => "127.0.0.1",
            Self::Localhost => "localhost",
        }
    }
}

/// Which port the loopback listener binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortRule {
    /// Any free port, chosen by the OS (Google, Microsoft).
    Any,
    /// The port the user's app registered; this is the default, and the
    /// user can change it with the client ID.
    Fixed(u16),
}

/// A provider's token paste rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSpec {
    /// What the provider calls it: "Personal access token", "API key".
    pub label: &'static str,
    /// A second field some providers need with the token (Jira's
    /// "Atlassian email"); `None` for a token alone.
    pub username_label: Option<&'static str>,
    /// How the token is sent.
    pub scheme: TokenScheme,
    /// Where the user creates the token.
    pub help_url: String,
}

/// How a pasted token is sent in `Authorization`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenScheme {
    /// `Bearer <token>` (most providers).
    Bearer,
    /// `<token>` alone (Linear and ClickUp keys).
    Bare,
    /// `Basic base64(<username>:<token>)` (Jira's email and API token).
    Basic,
}
