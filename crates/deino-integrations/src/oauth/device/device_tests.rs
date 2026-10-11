use std::sync::Arc;

use chrono::TimeZone;
use deino_domain::FakeClock;
use deino_net::{FakeNetClient, FakeReply, Status};

use super::*;
use crate::{ClientAuth, FakeSleeper, OAuthClient};

fn clock() -> Arc<FakeClock> {
    Arc::new(FakeClock::new(
        Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap(),
    ))
}

fn app() -> OAuthClient {
    OAuthClient {
        client_id: "c".into(),
        client_secret: None,
    }
}

fn pending(code: &str) -> FakeReply {
    FakeReply::status(Status::new(400)).body(format!(r#"{{"error":"{code}"}}"#))
}

const DEVICE: &str = r#"{"device_code":"dc","user_code":"WDJB-MJHT","verification_uri":"https://microsoft.com/devicelogin","expires_in":900,"interval":5}"#;

#[tokio::test]
async fn polls_through_pending_and_slow_down_until_approved() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(DEVICE));
    fake.reply(pending("authorization_pending"));
    fake.reply(pending("slow_down"));
    fake.reply(FakeReply::ok(r#"{"access_token":"at"}"#));
    let clock = clock();
    let sleeper = FakeSleeper::new(clock.clone());
    let app = app();
    let endpoint = TokenEndpoint {
        client: &fake,
        url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        auth: ClientAuth::Public,
        app: &app,
    };
    let start_at = clock.now();

    let device = start(
        &endpoint,
        "https://login.microsoftonline.com/devicecode",
        "Mail.Send",
        clock.as_ref(),
    )
    .await
    .unwrap();
    let pacing = Pacing {
        clock: clock.as_ref(),
        sleeper: &sleeper,
    };
    let tokens = poll(&endpoint, &device, &pacing, &CancellationToken::new())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(device.user_code, "WDJB-MJHT");
    assert_eq!(tokens.access_token.expose(), "at");
    let seconds: Vec<i64> = sleeper
        .waits()
        .iter()
        .map(|at| (*at - start_at).num_seconds())
        .collect();
    assert_eq!(seconds, [5, 10, 20]);
}

#[tokio::test]
async fn a_declined_code_and_an_expired_code_end_the_poll() {
    let fake = FakeNetClient::default();
    fake.reply(FakeReply::ok(DEVICE));
    fake.reply(pending("access_denied"));
    let clock = clock();
    let sleeper = FakeSleeper::new(clock.clone());
    let app = app();
    let endpoint = TokenEndpoint {
        client: &fake,
        url: "https://login.microsoftonline.com/token",
        auth: ClientAuth::Public,
        app: &app,
    };
    let device = start(
        &endpoint,
        "https://login.microsoftonline.com/devicecode",
        "s",
        clock.as_ref(),
    )
    .await
    .unwrap();
    let pacing = Pacing {
        clock: clock.as_ref(),
        sleeper: &sleeper,
    };

    let declined = poll(&endpoint, &device, &pacing, &CancellationToken::new()).await;
    clock.advance(TimeDelta::minutes(15));
    let expired = poll(&endpoint, &device, &pacing, &CancellationToken::new()).await;

    assert!(matches!(declined, Err(OAuthError::AccessDenied)));
    assert!(matches!(expired, Err(OAuthError::Expired)));
}
