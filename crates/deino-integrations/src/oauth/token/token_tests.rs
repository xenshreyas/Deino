use std::collections::BTreeMap;

use deino_net::{FakeNetClient, FakeReply, Status};

use super::*;

const URL: &str = "https://api.linear.app/oauth/token";

fn app(secret: Option<&str>) -> OAuthClient {
    OAuthClient {
        client_id: "client 1".into(),
        client_secret: secret.map(|secret| SecretValue::parse(secret).unwrap()),
    }
}

fn form(request: &Request) -> BTreeMap<String, String> {
    let body = request.body.clone().unwrap_or_default();
    form_urlencoded::parse(&body).into_owned().collect()
}

fn endpoint<'a>(
    fake: &'a FakeNetClient,
    app: &'a OAuthClient,
    auth: ClientAuth,
) -> TokenEndpoint<'a> {
    TokenEndpoint {
        client: fake,
        url: URL,
        auth,
        app,
    }
}

#[tokio::test]
async fn a_public_client_exchanges_the_code_with_its_verifier_and_no_secret() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(
        r#"{"access_token":"lin_a","token_type":"Bearer","expires_in":86399,
            "refresh_token":"lin_r","scope":"read write","api_domain":"https://x","n":1}"#,
    ));
    let app = app(None);

    let issued = endpoint(&fake, &app, ClientAuth::Public)
        .exchange("code-1", "http://localhost:53682/callback", "verifier-1")
        .await
        .unwrap();

    assert_eq!(issued.access_token.expose(), "lin_a");
    assert_eq!(issued.refresh_token.unwrap().expose(), "lin_r");
    assert_eq!(issued.expires_in, Some(Duration::from_secs(86_399)));
    assert_eq!(issued.scopes.unwrap(), ["read", "write"]);
    assert_eq!(issued.extras.get("api_domain"), Some("https://x"));
    assert_eq!(issued.extras.get("n"), None);
    let sent = fake.requests();
    let sent = form(&sent[0]);
    assert_eq!(sent["grant_type"], "authorization_code");
    assert_eq!(sent["code_verifier"], "verifier-1");
    assert_eq!(sent["client_id"], "client 1");
    assert!(!sent.contains_key("client_secret"));
}

#[tokio::test]
async fn a_secret_goes_in_the_body_or_a_basic_header_as_the_provider_wants() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(r#"{"access_token":"a"}"#));
    fake.reply(FakeReply::ok(r#"{"access_token":"b"}"#));
    let app = app(Some("s3cret:/"));

    endpoint(&fake, &app, ClientAuth::SecretInBody)
        .exchange("c", "http://127.0.0.1:1/callback", "v")
        .await
        .unwrap();
    endpoint(&fake, &app, ClientAuth::SecretInBasicHeader)
        .exchange("c", "http://127.0.0.1:1/callback", "v")
        .await
        .unwrap();

    let sent = fake.requests();
    assert_eq!(form(&sent[0])["client_secret"], "s3cret:/");
    let basic = sent[1].headers.get("authorization").unwrap();
    let decoded = STANDARD.decode(basic.trim_start_matches("Basic ")).unwrap();
    assert_eq!(String::from_utf8(decoded).unwrap(), "client+1:s3cret%3A%2F");
    assert!(!form(&sent[1]).contains_key("client_secret"));
}

#[tokio::test]
async fn refreshing_sends_the_refresh_token_and_reads_a_rotated_one() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(
        r#"{"access_token":"a2","refresh_token":"r2"}"#,
    ));
    let app = app(None);
    let old = SecretValue::parse("r1").unwrap();

    let issued = endpoint(&fake, &app, ClientAuth::Public)
        .refresh(&old)
        .await
        .unwrap();

    assert_eq!(issued.refresh_token.unwrap().expose(), "r2");
    let sent = form(&fake.requests()[0]);
    assert_eq!(sent["grant_type"], "refresh_token");
    assert_eq!(sent["refresh_token"], "r1");
}

#[tokio::test]
async fn a_dead_refresh_token_is_invalid_grant_and_other_codes_are_sanitized() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::status(Status::new(400)).body(r#"{"error":"invalid_grant"}"#));
    fake.reply(FakeReply::status(Status::new(400)).body(r#"{"error":"bad <b>scope</b>"}"#));
    fake.reply(FakeReply::status(Status::new(502)).body("<html>"));
    let app = app(None);
    let token = SecretValue::parse("r").unwrap();
    let endpoint = endpoint(&fake, &app, ClientAuth::Public);

    let dead = endpoint.refresh(&token).await.unwrap_err();
    let odd = endpoint.refresh(&token).await.unwrap_err();
    let html = endpoint.refresh(&token).await.unwrap_err();

    assert!(matches!(dead, OAuthError::InvalidGrant));
    assert!(matches!(odd, OAuthError::Provider(code) if code == "badbscopeb"));
    assert!(matches!(html, OAuthError::BadTokenResponse));
}

#[tokio::test]
async fn revoking_posts_the_token_to_the_revocation_endpoint() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(""));
    let app = app(None);
    let token = SecretValue::parse("lin_a").unwrap();

    endpoint(&fake, &app, ClientAuth::Public)
        .revoke("https://api.linear.app/oauth/revoke", &token)
        .await
        .unwrap();

    let sent = &fake.requests()[0];
    assert_eq!(sent.url.as_str(), "https://api.linear.app/oauth/revoke");
    assert_eq!(form(sent)["token"], "lin_a");
}

#[test]
fn the_client_debug_output_hides_the_secret() {
    let printed = format!("{:?}", app(Some("s3cret")));
    assert!(!printed.contains("s3cret"));
}
