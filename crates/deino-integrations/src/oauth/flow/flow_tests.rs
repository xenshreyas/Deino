//! The whole browser sign-in against a local authorization server: no
//! network beyond `127.0.0.1`, real sockets, real PKCE.

use std::collections::BTreeMap;

use deino_net::ReqwestNetClient;

use super::*;
use crate::testing::{MockAuthorizationServer, MockServerOptions, browser_get};
use crate::{FakeBrowser, LoopbackHost, PortRule, RedirectSpec, Scope};

const ANY_PORT: RedirectSpec = RedirectSpec {
    host: LoopbackHost::Ipv4,
    port: PortRule::Any,
    path: "/callback",
};

fn app() -> OAuthClient {
    OAuthClient {
        client_id: MockServerOptions::default().client_id,
        client_secret: None,
    }
}

#[tokio::test]
async fn a_user_who_approves_in_the_browser_gets_tokens_through_pkce() {
    let server = MockAuthorizationServer::start(MockServerOptions::default())
        .await
        .unwrap();
    let spec = server.spec(ANY_PORT);
    let (app, browser, client) = (
        app(),
        FakeBrowser::default(),
        ReqwestNetClient::new().unwrap(),
    );
    let flow = AuthorizationFlow {
        spec: &spec,
        app: &app,
        client: &client,
        browser: &browser,
        port: None,
    };
    let user = async {
        let opened = browser.next_opened().await.unwrap();
        let back = server.approve(&opened).unwrap();
        browser_get(&back).await.unwrap()
    };

    let cancel = CancellationToken::new();
    let (tokens, page) = tokio::join!(flow.run(&cancel), user);

    let tokens = tokens.unwrap().unwrap();
    assert!(tokens.access_token.expose().starts_with("at-"));
    assert!(tokens.refresh_token.is_some());
    assert_eq!(tokens.expires_in, Some(Duration::from_hours(1)));
    assert_eq!(tokens.extras.get("workspace_name"), Some("Mock workspace"));
    assert!(page.contains("close this tab"));
    assert_eq!(server.token_requests(), 1);
}

#[tokio::test]
async fn a_user_who_declines_ends_the_sign_in_without_a_token_request() {
    let server = MockAuthorizationServer::start(MockServerOptions::default())
        .await
        .unwrap();
    let spec = server.spec(ANY_PORT);
    let (app, browser, client) = (
        app(),
        FakeBrowser::default(),
        ReqwestNetClient::new().unwrap(),
    );
    let flow = AuthorizationFlow {
        spec: &spec,
        app: &app,
        client: &client,
        browser: &browser,
        port: None,
    };
    let user = async {
        let opened = browser.next_opened().await.unwrap();
        let back = MockAuthorizationServer::deny(&opened).unwrap();
        browser_get(&back).await.unwrap()
    };

    let cancel = CancellationToken::new();
    let (outcome, _) = tokio::join!(flow.run(&cancel), user);

    assert!(matches!(outcome, Err(OAuthError::AccessDenied)));
    assert_eq!(server.token_requests(), 0);
}

#[tokio::test]
async fn a_browser_that_cannot_open_fails_before_waiting() {
    let server = MockAuthorizationServer::start(MockServerOptions::default())
        .await
        .unwrap();
    let spec = server.spec(ANY_PORT);
    let (app, browser, client) = (
        app(),
        FakeBrowser::default(),
        ReqwestNetClient::new().unwrap(),
    );
    browser.break_it();
    let flow = AuthorizationFlow {
        spec: &spec,
        app: &app,
        client: &client,
        browser: &browser,
        port: None,
    };

    let outcome = flow.run(&CancellationToken::new()).await;

    assert!(matches!(outcome, Err(OAuthError::Browser)));
}

#[test]
fn the_authorization_url_carries_pkce_state_scopes_and_extras() {
    let spec = OAuthSpec {
        scopes: vec![
            Scope::new("chat:write", "Post"),
            Scope::new("users:read", "People"),
        ],
        scope_param: "user_scope",
        scope_separator: ',',
        ..test_spec()
    };
    let redirect = Url::parse("http://localhost:53682/callback").unwrap();

    let url = authorization_url(&AuthorizationRequest {
        spec: &spec,
        client_id: "123.456",
        redirect_uri: &redirect,
        state: "st",
        challenge: "ch",
    })
    .unwrap();

    let query: BTreeMap<String, String> = url.query_pairs().into_owned().collect();
    assert_eq!(url.host_str(), Some("slack.com"));
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["client_id"], "123.456");
    assert_eq!(query["redirect_uri"], "http://localhost:53682/callback");
    assert_eq!(query["user_scope"], "chat:write,users:read");
    assert_eq!(query["state"], "st");
    assert_eq!(query["code_challenge"], "ch");
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["access_type"], "offline");
    assert!(!query.contains_key("scope"));
}

fn test_spec() -> OAuthSpec {
    OAuthSpec {
        authorize_url: "https://slack.com/oauth/v2/authorize".into(),
        token_url: "https://slack.com/api/oauth.v2.access".into(),
        revoke_url: None,
        scopes: Vec::new(),
        scope_param: "scope",
        scope_separator: ' ',
        extra_params: vec![("access_type", "offline")],
        redirect: ANY_PORT,
        client_auth: crate::ClientAuth::Public,
        app_setup_url: "https://api.slack.com/apps".into(),
    }
}
