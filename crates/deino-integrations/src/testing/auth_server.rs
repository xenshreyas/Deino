//! [`MockAuthorizationServer`]: an OAuth 2.0 authorization server on
//! `127.0.0.1` that checks what a real one checks: the client ID, the
//! redirect URI, single-use codes, the PKCE verifier against its S256
//! challenge, refresh token rotation (reusing a rotated token revokes the
//! whole grant, as Todoist does), and RFC 7009 revocation. It also plays a
//! tiny provider API (`GET /me`, `POST /messages`) that accepts only live
//! access tokens, so tests see a 401 once the tokens are expired.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;
use url::{Url, form_urlencoded};

use super::http::{ReceivedRequest, read_request, write_json};
use crate::{ClientAuth, OAuthSpec, Pkce, RedirectSpec, Scope};

/// How the server behaves.
#[derive(Clone, Debug)]
pub struct MockServerOptions {
    /// The only client ID it accepts.
    pub client_id: String,
    /// `expires_in` of every access token.
    pub expires_in: u64,
    /// Issue a new refresh token on every refresh.
    pub rotate_refresh_tokens: bool,
}

impl Default for MockServerOptions {
    fn default() -> Self {
        Self {
            client_id: "deino-test-client".into(),
            expires_in: 3600,
            rotate_refresh_tokens: true,
        }
    }
}

/// A code the user approved, waiting for its exchange.
#[derive(Clone, Debug)]
struct PendingCode {
    challenge: String,
    redirect_uri: String,
}

#[derive(Debug, Default)]
struct State {
    codes: BTreeMap<String, PendingCode>,
    live_access: BTreeSet<String>,
    live_refresh: BTreeSet<String>,
    spent_refresh: BTreeSet<String>,
    revoked: Vec<String>,
    issued: u32,
    token_requests: u32,
}

/// The running server. Dropping it stops it.
pub struct MockAuthorizationServer {
    base: Url,
    options: MockServerOptions,
    state: Arc<Mutex<State>>,
    _tasks: JoinSet<()>,
}

impl MockAuthorizationServer {
    /// Starts a server on an ephemeral `127.0.0.1` port.
    pub async fn start(options: MockServerOptions) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let base = Url::parse(&format!("http://{address}/"))
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let state = Arc::new(Mutex::new(State::default()));
        let mut tasks = JoinSet::new();
        tasks.spawn(serve(listener, options.clone(), Arc::clone(&state)));
        Ok(Self {
            base,
            options,
            state,
            _tasks: tasks,
        })
    }

    /// `http://127.0.0.1:<port>/`: the provider API's base too.
    pub fn base(&self) -> &Url {
        &self.base
    }

    /// OAuth settings that point at this server, redirecting per `redirect`.
    pub fn spec(&self, redirect: RedirectSpec) -> OAuthSpec {
        let at = |path: &str| self.base.join(path).map(String::from).unwrap_or_default();
        OAuthSpec {
            authorize_url: at("authorize"),
            token_url: at("token"),
            revoke_url: Some(at("revoke")),
            scopes: vec![Scope::new("write", "Post on your behalf")],
            scope_param: "scope",
            scope_separator: ' ',
            extra_params: vec![("access_type", "offline")],
            redirect,
            client_auth: ClientAuth::Public,
            app_setup_url: at("apps"),
        }
    }

    /// The user approves `authorize_url` in the browser: returns where the
    /// browser is sent back to, or `None` if the request is malformed.
    pub fn approve(&self, authorize_url: &Url) -> Option<Url> {
        let query: BTreeMap<String, String> = authorize_url.query_pairs().into_owned().collect();
        let valid = query.get("response_type")? == "code"
            && query.get("client_id")? == &self.options.client_id
            && query.get("code_challenge_method")? == "S256";
        if !valid {
            return None;
        }
        let redirect_uri = query.get("redirect_uri")?.clone();
        let mut state = lock(&self.state);
        state.issued += 1;
        let code = format!("code-{}", state.issued);
        let pending = PendingCode {
            challenge: query.get("code_challenge")?.clone(),
            redirect_uri: redirect_uri.clone(),
        };
        state.codes.insert(code.clone(), pending);
        let mut back = Url::parse(&redirect_uri).ok()?;
        back.query_pairs_mut()
            .append_pair("code", &code)
            .append_pair("state", query.get("state")?);
        Some(back)
    }

    /// The user declines: where the browser is sent back to.
    pub fn deny(authorize_url: &Url) -> Option<Url> {
        let query: BTreeMap<String, String> = authorize_url.query_pairs().into_owned().collect();
        let mut back = Url::parse(query.get("redirect_uri")?).ok()?;
        back.query_pairs_mut()
            .append_pair("error", "access_denied")
            .append_pair("state", query.get("state")?);
        Some(back)
    }

    /// Every token revoked through the revocation endpoint.
    pub fn revoked(&self) -> Vec<String> {
        lock(&self.state).revoked.clone()
    }

    /// Expires every access token issued so far, as the provider's clock
    /// would: the API answers 401 until a refresh.
    pub fn expire_access_tokens(&self) {
        lock(&self.state).live_access.clear();
    }

    /// How many token requests (exchanges and refreshes) arrived.
    pub fn token_requests(&self) -> u32 {
        lock(&self.state).token_requests
    }
}

async fn serve(listener: TcpListener, options: MockServerOptions, state: Arc<Mutex<State>>) {
    let mut connections = JoinSet::new();
    while let Ok((stream, _)) = listener.accept().await {
        connections.spawn(answer(stream, options.clone(), Arc::clone(&state)));
    }
}

async fn answer(mut stream: TcpStream, options: MockServerOptions, state: Arc<Mutex<State>>) {
    let Ok(request) = read_request(&mut stream).await else {
        return;
    };
    let (status, body) = respond(&request, &options, &mut lock(&state));
    if let Err(error) = write_json(&mut stream, status, &body).await {
        log::debug!(target: "integrations", "mock server could not answer: {error}");
    }
}

fn respond(
    request: &ReceivedRequest,
    options: &MockServerOptions,
    state: &mut State,
) -> (u16, String) {
    let form: BTreeMap<String, String> =
        form_urlencoded::parse(&request.body).into_owned().collect();
    let field = |name: &str| form.get(name).map(String::as_str).unwrap_or_default();
    match request.path() {
        "/revoke" => {
            state.revoked.push(field("token").to_owned());
            (200, "{}".into())
        }
        "/me" | "/messages" => api(request, state),
        "/token" if field("client_id") != options.client_id => error("invalid_client"),
        "/token" => {
            state.token_requests += 1;
            match field("grant_type") {
                "authorization_code" => exchange(&form, options, state),
                "refresh_token" => refresh(field("refresh_token"), options, state),
                _ => error("unsupported_grant_type"),
            }
        }
        _ => (404, "{}".into()),
    }
}

/// The provider API: only a live `Bearer` token gets an answer.
fn api(request: &ReceivedRequest, state: &mut State) -> (u16, String) {
    let token = request
        .header("authorization")
        .and_then(|value| value.strip_prefix("bearer "));
    if !token.is_some_and(|token| state.live_access.contains(token)) {
        return (401, r#"{"error":"invalid_auth"}"#.into());
    }
    state.issued += 1;
    match request.path() {
        "/me" => (
            200,
            r#"{"id":"mock-account","name":"Mock workspace"}"#.into(),
        ),
        _ => (200, format!(r#"{{"id":"msg-{}"}}"#, state.issued)),
    }
}

fn exchange(
    form: &BTreeMap<String, String>,
    options: &MockServerOptions,
    state: &mut State,
) -> (u16, String) {
    let field = |name: &str| form.get(name).map(String::as_str).unwrap_or_default();
    let Some(pending) = state.codes.remove(field("code")) else {
        return error("invalid_grant");
    };
    let verified = Pkce::from_verifier(field("code_verifier")).challenge() == pending.challenge;
    if !verified || field("redirect_uri") != pending.redirect_uri {
        return error("invalid_grant");
    }
    issue(options, state)
}

fn refresh(token: &str, options: &MockServerOptions, state: &mut State) -> (u16, String) {
    if state.spent_refresh.contains(token) {
        // A reused rotated token means it leaked: revoke the whole grant.
        state.live_refresh.clear();
        return error("invalid_grant");
    }
    if !state.live_refresh.contains(token) {
        return error("invalid_grant");
    }
    if options.rotate_refresh_tokens {
        state.live_refresh.remove(token);
        state.spent_refresh.insert(token.to_owned());
    }
    issue(options, state)
}

fn issue(options: &MockServerOptions, state: &mut State) -> (u16, String) {
    state.issued += 1;
    let refresh = format!("rt-{}", state.issued);
    state.live_refresh.insert(refresh.clone());
    state.live_access.insert(format!("at-{}", state.issued));
    let body = format!(
        r#"{{"access_token":"at-{}","token_type":"Bearer","expires_in":{},"refresh_token":"{refresh}","scope":"write","workspace_name":"Mock workspace"}}"#,
        state.issued, options.expires_in
    );
    (200, body)
}

fn error(code: &str) -> (u16, String) {
    (400, format!(r#"{{"error":"{code}"}}"#))
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}
