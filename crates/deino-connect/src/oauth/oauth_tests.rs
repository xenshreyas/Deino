//! The OAuth client against scripted token endpoints, and a whole sign-in
//! through a real loopback socket and a fake browser.

use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::FakeClock;
use deino_net::{FakeNetClient, FakeReply, Status};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

use super::*;
use crate::fake::FakeBrowser;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-10-09T09:00:00Z")
        .unwrap()
        .to_utc()
}

fn google() -> OAuthClient {
    OAuthClient::new(
        &GOOGLE,
        "client-1.apps.googleusercontent.com".into(),
        Some("shh".into()),
    )
}

fn microsoft() -> OAuthClient {
    OAuthClient::new(&MICROSOFT, "00000000-aaaa".into(), None)
}

fn form_of(fake: &FakeNetClient) -> Vec<(String, String)> {
    let request = fake.requests().pop().unwrap();
    url::form_urlencoded::parse(&request.body.unwrap())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

#[test]
fn the_sign_in_page_asks_for_read_only_access_with_an_s256_challenge() {
    let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into());

    let page = google()
        .authorize_url("http://127.0.0.1:5000/callback", &pkce, "st")
        .unwrap();

    let query: Vec<(String, String)> = page
        .query_pairs()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
    let value = |key: &str| {
        query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(page.host_str(), Some("accounts.google.com"));
    assert_eq!(
        value("scope"),
        Some("https://www.googleapis.com/auth/calendar.readonly")
    );
    assert_eq!(
        value("code_challenge"),
        Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM")
    );
    assert_eq!(value("code_challenge_method"), Some("S256"));
    assert_eq!(value("access_type"), Some("offline"));
    assert_eq!(value("state"), Some("st"));
    assert!(
        !page.as_str().contains("shh"),
        "the secret never goes to the browser"
    );
}

#[tokio::test]
async fn a_code_is_traded_with_its_verifier_and_google_gets_the_client_secret() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(
        r#"{"access_token":"at","refresh_token":"rt","expires_in":3599}"#,
    ));
    let grant = Grant::Code {
        code: "c",
        redirect_uri: "http://127.0.0.1:1/callback",
        verifier: "v",
    };

    let tokens = google().exchange(&fake, grant, now()).await.unwrap();

    assert_eq!(tokens.refresh_token.as_deref(), Some("rt"));
    assert_eq!(tokens.expires_at, now() + TimeDelta::seconds(3599));
    let form = form_of(&fake);
    assert!(form.contains(&("grant_type".into(), "authorization_code".into())));
    assert!(form.contains(&("code_verifier".into(), "v".into())));
    assert!(form.contains(&("client_secret".into(), "shh".into())));
}

#[tokio::test]
async fn microsoft_refreshes_without_a_secret_and_a_refused_refresh_means_reconnect() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(
        r#"{"access_token":"at2","refresh_token":"rt2","expires_in":4000}"#,
    ));
    fake.reply(FakeReply::ok(r#"{"error":"invalid_client"}"#).with_status(Status::new(401)));

    let refreshed = microsoft()
        .exchange(&fake, Grant::Refresh("rt"), now())
        .await
        .unwrap();
    let form = form_of(&fake);

    assert_eq!(refreshed.access_token, "at2");
    assert!(!form.iter().any(|(key, _)| key == "client_secret"));
    assert!(form.contains(&("refresh_token".into(), "rt".into())));
    let refused = microsoft()
        .exchange(&fake, Grant::Refresh("rt2"), now())
        .await;
    assert!(
        matches!(refused, Err(OAuthError::Denied { reason, .. }) if reason == "invalid_client")
    );
}

#[tokio::test]
async fn an_invalid_grant_on_refresh_is_a_revoked_connection() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(r#"{"error":"invalid_grant"}"#).with_status(Status::new(400)));

    let outcome = google().exchange(&fake, Grant::Refresh("old"), now()).await;

    assert!(matches!(outcome, Err(OAuthError::Revoked("Google"))));
}

/// Plays the browser: waits for the sign-in page, then follows the redirect
/// with the page's state, as the provider would after the user approves.
async fn approve(browser: &FakeBrowser) {
    let page = loop {
        if let Some(page) = browser.opened().pop() {
            break Url::parse(&page).unwrap();
        }
        tokio::task::yield_now().await;
    };
    let value = |key: &str| {
        page.query_pairs()
            .find(|(k, _)| k == key)
            .unwrap()
            .1
            .into_owned()
    };
    let redirect = Url::parse(&value("redirect_uri")).unwrap();
    let mut stream = TcpStream::connect(("127.0.0.1", redirect.port().unwrap()))
        .await
        .unwrap();
    let request = format!(
        "GET /callback?code=the-code&state={} HTTP/1.1\r\n\r\n",
        value("state")
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).await.unwrap();
}

#[tokio::test]
async fn signing_in_opens_the_browser_receives_the_code_and_returns_tokens() {
    let (fake, browser, clock) = (
        FakeNetClient::default(),
        FakeBrowser::new(),
        FakeClock::new(now()),
    );
    fake.reply(FakeReply::ok(
        r#"{"access_token":"at","refresh_token":"rt","expires_in":3600}"#,
    ));
    let client = microsoft();
    let flow = SignIn {
        client: &client,
        http: &fake,
        browser: &browser,
        clock: &clock,
    };

    let (tokens, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(sign_in(flow), approve(&browser))
    })
    .await
    .unwrap();

    assert_eq!(tokens.unwrap().refresh_token.as_deref(), Some("rt"));
    assert!(browser.opened()[0].contains("redirect_uri=http%3A%2F%2Flocalhost%3A"));
    assert!(form_of(&fake).contains(&("code".into(), "the-code".into())));
}

#[tokio::test]
async fn without_a_browser_nothing_is_requested() {
    let (fake, browser, clock) = (
        FakeNetClient::default(),
        FakeBrowser::broken(),
        FakeClock::new(now()),
    );
    let client = google();

    let outcome = sign_in(SignIn {
        client: &client,
        http: &fake,
        browser: &browser,
        clock: &clock,
    })
    .await;

    assert!(matches!(outcome, Err(OAuthError::BrowserUnavailable)));
    assert!(fake.requests().is_empty());
}
