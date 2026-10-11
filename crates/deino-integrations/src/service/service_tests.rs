//! The lifecycle and the delivery engine over the real `deino-net` scoping
//! and policy, with a scripted transport.

use chrono::TimeDelta;
use deino_domain::{
    AuthMethod, Capability, ConnectionStatus, DeliveryStatus, ErrorCode, IntegrationProvider,
    ServiceId,
};
use deino_net::{EgressOutcome, FakeReply, ServicePolicy, Status};

use super::test_harness::{harness, linear, message, scripted_net, stored_text};
use super::*;
use crate::{Action, ConnectMethod, DeliveryRequest, PageDraft, TokenInput};

const ME: &str = r#"{"id":"org-1","name":"Acme · ana@acme.com"}"#;
const API_KEY: &str = "lin_api_s3cret";

fn key(token: &str) -> TokenInput {
    TokenInput {
        token: token.into(),
        username: None,
    }
}

/// A harness with a connected Linear-like account.
async fn connected() -> (
    super::test_harness::Harness,
    super::test_harness::ScriptedNet,
    Connection,
) {
    let net = scripted_net();
    let harness = harness(vec![linear()], net.egress.clone()).await;
    net.fake.reply(FakeReply::ok(ME));
    let connection = harness
        .service
        .connect_token(IntegrationProvider::Linear, key(API_KEY))
        .await
        .unwrap();
    (harness, net, connection)
}

#[tokio::test]
async fn a_pasted_key_is_identified_kept_in_the_keychain_and_switches_the_service_on() {
    let (harness, net, connection) = connected().await;

    assert_eq!(connection.account_label, "Acme · ana@acme.com");
    assert_eq!(connection.auth_method, AuthMethod::Token);
    assert_eq!(connection.status, ConnectionStatus::Connected);
    let sent = net.fake.requests();
    assert_eq!(sent[0].url.as_str(), "https://api.linear.app/me");
    assert_eq!(sent[0].headers.get("authorization"), Some(API_KEY));
    assert_eq!(harness.store.keys().len(), 1);
    assert!(!stored_text(&harness.database).await.contains(API_KEY));
    assert!(net.policy.grant(ServiceId::Linear).await.enabled);
}

#[tokio::test]
async fn offline_mode_stops_a_connect_before_anything_is_sent() {
    let net = scripted_net();
    let harness = harness(vec![linear()], net.egress.clone()).await;
    net.policy.set_offline(true);

    let error = harness
        .service
        .connect_token(IntegrationProvider::Linear, key(API_KEY))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "offline mode is on, so Linear cannot be reached"
    );
    assert!(net.fake.requests().is_empty());
    assert!(harness.store.keys().is_empty());
}

#[tokio::test]
async fn a_rejected_key_leaves_nothing_behind_and_the_service_off() {
    let net = scripted_net();
    let harness = harness(vec![linear()], net.egress.clone()).await;
    net.fake.reply(FakeReply::status(Status::new(401)));

    let error = harness
        .service
        .connect_token(IntegrationProvider::Linear, key("wrong"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        IntegrationError::NeedsReconnect(IntegrationProvider::Linear)
    ));
    assert!(harness.store.keys().is_empty());
    assert!(
        harness.service.providers().await.unwrap()[0]
            .connections
            .is_empty()
    );
    assert!(!net.policy.grant(ServiceId::Linear).await.enabled);
}

#[tokio::test]
async fn a_delivery_is_sent_once_per_idempotency_key() {
    let (harness, net, connection) = connected().await;
    net.fake.reply(FakeReply::ok(
        r#"{"id":"msg-1","url":"https://linear.app/m/1"}"#,
    ));

    let first = harness
        .service
        .deliver(message(connection.id, "share:m1:c1"))
        .await
        .unwrap();
    let again = harness
        .service
        .deliver(message(connection.id, "share:m1:c1"))
        .await
        .unwrap();

    assert_eq!(first.status, DeliveryStatus::Succeeded);
    assert_eq!(first.external.as_ref().unwrap().id, "msg-1");
    assert_eq!(again, first);
    let posts = net
        .fake
        .requests()
        .iter()
        .filter(|r| r.url.path() == "/messages")
        .count();
    assert_eq!(posts, 1);
    let sent = net.fake.requests();
    assert_eq!(sent[1].headers.get("idempotency-key"), Some("share:m1:c1"));
    assert_eq!(harness.service.deliveries(10).await.unwrap(), [first]);
}

#[tokio::test]
async fn offline_mode_fails_a_delivery_cleanly_and_logs_why() {
    let (harness, net, connection) = connected().await;
    net.policy.set_offline(true);

    let error = harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        IntegrationError::Offline(IntegrationProvider::Linear)
    ));
    let log = harness.service.deliveries(10).await.unwrap();
    assert_eq!(log[0].status, DeliveryStatus::Failed);
    assert_eq!(
        log[0].error.as_ref().unwrap().message,
        "offline mode is on, so Linear cannot be reached"
    );
    assert_eq!(
        net.fake.requests().len(),
        1,
        "only the connect's identify was sent"
    );
    let connection = harness.service.providers().await.unwrap()[0].connections[0].clone();
    assert_eq!(
        connection.last_error, None,
        "offline is not the connection's fault"
    );
}

#[tokio::test]
async fn a_switched_off_service_is_blocked_by_policy_and_logged_as_blocked() {
    let (harness, net, connection) = connected().await;
    deino_net::ServiceSwitch::set_enabled(net.policy.as_ref(), ServiceId::Linear, false)
        .await
        .unwrap();

    let error = harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        IntegrationError::Disabled(IntegrationProvider::Linear)
    ));
    let blocked = deino_net::EgressReader::recent(net.egress_log.as_ref(), 10)
        .await
        .unwrap();
    assert!(
        blocked
            .iter()
            .any(|entry| entry.outcome == EgressOutcome::Blocked)
    );
}

#[tokio::test]
async fn a_provider_specific_rate_limit_is_waited_out_and_retried() {
    let (harness, net, connection) = connected().await;
    let start = harness.clock.now();
    net.fake.reply(
        FakeReply::status(Status::new(400))
            .body(r#"{"errors":[{"extensions":{"code":"RATELIMITED"}}]}"#),
    );
    net.fake.reply(FakeReply::ok(r#"{"id":"msg-2"}"#));

    let delivery = harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap();

    assert_eq!(delivery.status, DeliveryStatus::Succeeded);
    assert_eq!(delivery.attempts, 1);
    assert!(
        harness
            .sleeper
            .waits()
            .contains(&(start + TimeDelta::seconds(2)))
    );
}

#[tokio::test]
async fn an_outage_is_retried_with_backoff_then_left_failed_for_a_later_attempt() {
    let (harness, net, connection) = connected().await;
    let start = harness.clock.now();
    for _ in 0..3 {
        net.fake.reply(FakeReply::status(Status::new(503)));
    }

    let error = harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        IntegrationError::Unavailable(IntegrationProvider::Linear)
    ));
    let waits = harness.sleeper.waits();
    assert!(waits.contains(&(start + TimeDelta::seconds(2))));
    assert!(waits.contains(&(start + TimeDelta::seconds(6))));
    let log = harness.service.deliveries(1).await.unwrap();
    assert_eq!(log[0].status, DeliveryStatus::Failed);
    assert_eq!(log[0].error.as_ref().unwrap().code, ErrorCode::Network);
    net.fake.reply(FakeReply::ok(r#"{"id":"msg-9"}"#));
    let resumed = harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap();
    assert_eq!(
        (resumed.status, resumed.attempts),
        (DeliveryStatus::Succeeded, 2)
    );
}

#[tokio::test]
async fn a_revoked_key_marks_the_connection_for_reconnecting() {
    let (harness, net, connection) = connected().await;
    net.fake.reply(FakeReply::status(Status::new(401)));

    let error = harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        IntegrationError::NeedsReconnect(IntegrationProvider::Linear)
    ));
    let view = harness.service.providers().await.unwrap();
    let stored = &view[0].connections[0];
    assert_eq!(stored.status, ConnectionStatus::NeedsReconnect);
    assert_eq!(
        stored.last_error.as_ref().unwrap().code,
        ErrorCode::PermissionDenied
    );
}

#[tokio::test]
async fn a_capability_the_provider_lacks_is_refused_before_anything_is_recorded() {
    let (harness, _net, connection) = connected().await;
    let request = DeliveryRequest {
        action: Action::CreatePage(PageDraft {
            parent: "p".into(),
            title: "t".into(),
            body: "b".into(),
        }),
        ..message(connection.id, "k")
    };

    let error = harness.service.deliver(request).await.unwrap_err();

    assert_eq!(error.to_string(), "Linear cannot create page");
    assert!(harness.service.deliveries(10).await.unwrap().is_empty());
}

#[tokio::test]
async fn targets_come_from_the_provider() {
    let (harness, net, connection) = connected().await;
    net.fake
        .reply(FakeReply::ok(r#"[{"id":"T1","name":"Platform"}]"#));

    let teams = harness
        .service
        .targets(connection.id, Capability::CreateTask)
        .await
        .unwrap();

    assert_eq!(teams[0].name, "Platform");
}

#[tokio::test]
async fn disconnecting_forgets_the_key_keeps_the_log_and_switches_the_service_off() {
    let (harness, net, connection) = connected().await;
    net.fake.reply(FakeReply::ok(r#"{"id":"msg-1"}"#));
    harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap();

    harness.service.disconnect(connection.id).await.unwrap();

    assert!(harness.store.keys().is_empty());
    assert!(
        harness.service.providers().await.unwrap()[0]
            .connections
            .is_empty()
    );
    assert_eq!(
        harness.service.deliveries(10).await.unwrap()[0].connection_id,
        None
    );
    assert!(!net.policy.grant(ServiceId::Linear).await.enabled);
    let missing = harness.service.disconnect(connection.id).await.unwrap_err();
    assert!(matches!(missing, IntegrationError::ConnectionNotFound(_)));
}

#[tokio::test]
async fn the_provider_view_explains_how_to_connect() {
    let net = scripted_net();
    let harness = harness(vec![linear()], net.egress.clone()).await;

    let view = harness.service.providers().await.unwrap();

    assert_eq!(view[0].name, "Linear");
    assert_eq!(
        view[0].capabilities,
        [Capability::PostMessage, Capability::CreateTask]
    );
    assert!(matches!(
        &view[0].methods[0],
        ConnectMethod::Token { label, username_label: None, .. } if label == "API key"
    ));
    assert_eq!(view[0].app, None);
}

#[tokio::test]
async fn an_app_needs_oauth_and_a_client_id() {
    let net = scripted_net();
    let harness = harness(vec![linear()], net.egress.clone()).await;
    let app = crate::AppInput {
        client_id: "abc".into(),
        client_secret: None,
        redirect_port: None,
    };

    let error = harness
        .service
        .set_app(IntegrationProvider::Linear, app)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        IntegrationError::MethodUnavailable(IntegrationProvider::Linear)
    ));
    assert!(
        harness
            .apps
            .registration(IntegrationProvider::Linear)
            .await
            .is_none()
    );
}
