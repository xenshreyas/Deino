//! Connecting in the browser and refreshing tokens, end to end against a
//! local authorization server (real sockets on 127.0.0.1, real PKCE, token
//! rotation with reuse detection).

use std::sync::Arc;

use chrono::TimeDelta;
use deino_domain::{AuthMethod, ConnectionStatus, IntegrationProvider, ServiceId};
use deino_net::ReqwestNetClient;

use super::test_harness::{Harness, harness, message};
use super::*;
use crate::testing::{MockAuthorizationServer, MockServerOptions, browser_get};
use crate::{
    AppRegistration, AuthSpec, Credential, FakeEgress, LoopbackHost, OAuthApps, PortRule,
    RedirectSpec, ScriptedIntegration,
};

const REDIRECT: RedirectSpec = RedirectSpec {
    host: LoopbackHost::Ipv4,
    port: PortRule::Any,
    path: "/callback",
};

struct OAuthHarness {
    harness: Harness,
    server: MockAuthorizationServer,
    egress: Arc<FakeEgress>,
}

/// A provider whose OAuth server and API are the local mock, with the
/// user's app registered.
async fn oauth_harness() -> OAuthHarness {
    let server = MockAuthorizationServer::start(MockServerOptions::default())
        .await
        .unwrap();
    let auth = vec![AuthSpec::OAuth(server.spec(REDIRECT))];
    let provider =
        ScriptedIntegration::new(IntegrationProvider::Slack, auth, server.base().as_str());
    let egress = Arc::new(FakeEgress::new(Arc::new(ReqwestNetClient::new().unwrap())));
    let harness = harness(vec![Arc::new(provider)], egress.clone()).await;
    let registration = AppRegistration {
        client_id: MockServerOptions::default().client_id,
        redirect_port: None,
    };
    harness
        .apps
        .save(IntegrationProvider::Slack, Some(registration))
        .await
        .unwrap();
    OAuthHarness {
        harness,
        server,
        egress,
    }
}

/// Connects, approving in the "browser".
async fn connect(setup: &OAuthHarness) -> Connection {
    let user = async {
        let opened = setup.harness.browser.next_opened().await.unwrap();
        browser_get(&setup.server.approve(&opened).unwrap())
            .await
            .unwrap()
    };
    let (connection, _) = tokio::join!(
        setup.harness.service.connect(IntegrationProvider::Slack),
        user
    );
    connection.unwrap()
}

#[tokio::test]
async fn connecting_in_the_browser_stores_the_account_and_keeps_tokens_in_the_keychain() {
    let setup = oauth_harness().await;

    let connection = connect(&setup).await;

    let now = setup.harness.clock.now();
    assert_eq!(connection.account_label, "Mock workspace");
    assert_eq!(connection.auth_method, AuthMethod::OAuth);
    assert_eq!(connection.scopes, ["write"]);
    assert_eq!(connection.token_expires_at, Some(now + TimeDelta::hours(1)));
    assert_eq!(setup.egress.switches().get(&ServiceId::Slack), Some(&true));
    let stored = setup
        .harness
        .store
        .stored(&setup.harness.store.keys()[0])
        .unwrap();
    assert!(
        stored.contains("rt-"),
        "the refresh token is in the Keychain"
    );
    let text = super::test_harness::stored_text(&setup.harness.database).await;
    assert!(!text.contains("at-") && !text.contains("rt-"), "{text}");
}

#[tokio::test]
async fn connecting_needs_the_users_app_first() {
    let setup = oauth_harness().await;
    setup
        .harness
        .apps
        .save(IntegrationProvider::Slack, None)
        .await
        .unwrap();

    let error = setup
        .harness
        .service
        .connect(IntegrationProvider::Slack)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "add your Slack app's client ID first");
}

#[tokio::test]
async fn cancelling_a_waiting_sign_in_switches_the_service_back_off() {
    let setup = oauth_harness().await;
    let user = async {
        setup.harness.browser.next_opened().await.unwrap();
        assert!(
            setup
                .harness
                .service
                .cancel_connect(IntegrationProvider::Slack)
        );
    };

    let (outcome, ()) = tokio::join!(
        setup.harness.service.connect(IntegrationProvider::Slack),
        user
    );

    assert!(matches!(
        outcome,
        Err(IntegrationError::Cancelled(IntegrationProvider::Slack))
    ));
    assert_eq!(setup.egress.switches().get(&ServiceId::Slack), Some(&false));
    assert!(
        !setup
            .harness
            .service
            .cancel_connect(IntegrationProvider::Slack)
    );
}

#[tokio::test]
async fn an_expiring_token_is_refreshed_before_sending_and_the_rotated_pair_is_kept() {
    let setup = oauth_harness().await;
    let connection = connect(&setup).await;
    setup.harness.clock.advance(TimeDelta::minutes(59));

    let delivery = setup
        .harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap();

    assert!(delivery.external.unwrap().id.starts_with("msg-"));
    assert_eq!(setup.server.token_requests(), 2);
    let vault = &setup.harness.service.ports.vault;
    let credential = vault
        .load(IntegrationProvider::Slack, connection.id)
        .await
        .unwrap();
    let Some(Credential::OAuth {
        refresh: Some(refresh),
        ..
    }) = credential
    else {
        panic!("expected rotated OAuth tokens");
    };
    assert!(refresh.expose().starts_with("rt-"));
    let stored = setup
        .harness
        .service
        .connection(connection.id)
        .await
        .unwrap();
    assert_eq!(
        stored.token_expires_at,
        Some(setup.harness.clock.now() + TimeDelta::hours(1))
    );
}

#[tokio::test]
async fn concurrent_sends_refresh_once_so_a_rotated_token_is_never_reused() {
    let setup = oauth_harness().await;
    let connection = connect(&setup).await;
    setup.harness.clock.advance(TimeDelta::hours(2));
    let service = &setup.harness.service;

    let (first, second) = tokio::join!(
        service.deliver(message(connection.id, "a")),
        service.deliver(message(connection.id, "b")),
    );

    assert!(first.is_ok() && second.is_ok(), "{first:?} {second:?}");
    assert_eq!(
        setup.server.token_requests(),
        2,
        "one exchange, one refresh"
    );
}

#[tokio::test]
async fn a_401_refreshes_once_and_retries() {
    let setup = oauth_harness().await;
    let connection = connect(&setup).await;
    setup.server.expire_access_tokens();

    let delivery = setup
        .harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap();

    assert!(delivery.external.is_some());
    assert_eq!(setup.server.token_requests(), 2);
}

#[tokio::test]
async fn a_dead_refresh_token_asks_the_user_to_connect_again() {
    let setup = oauth_harness().await;
    let connection = connect(&setup).await;
    let vault = &setup.harness.service.ports.vault;
    let original = vault
        .load(IntegrationProvider::Slack, connection.id)
        .await
        .unwrap()
        .unwrap();
    // The first refresh token is spent once; reusing it later makes the
    // server revoke the whole grant, as Todoist and Linear do.
    setup.harness.clock.advance(TimeDelta::hours(2));
    setup.harness.service.check(connection.id).await.unwrap();
    vault
        .save(IntegrationProvider::Slack, connection.id, &original)
        .await
        .unwrap();
    setup.harness.clock.advance(TimeDelta::hours(2));

    let error = setup
        .harness
        .service
        .deliver(message(connection.id, "k"))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        IntegrationError::NeedsReconnect(IntegrationProvider::Slack)
    ));
    let stored = setup
        .harness
        .service
        .connection(connection.id)
        .await
        .unwrap();
    assert_eq!(stored.status, ConnectionStatus::NeedsReconnect);
}

#[tokio::test]
async fn disconnecting_revokes_the_grant_at_the_provider() {
    let setup = oauth_harness().await;
    let connection = connect(&setup).await;

    setup
        .harness
        .service
        .disconnect(connection.id)
        .await
        .unwrap();

    let revoked = setup.server.revoked();
    assert_eq!(revoked.len(), 1);
    assert!(
        revoked[0].starts_with("rt-"),
        "the refresh token ends the grant"
    );
    assert_eq!(setup.egress.switches().get(&ServiceId::Slack), Some(&false));
}
