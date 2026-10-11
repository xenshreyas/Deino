//! The connector's token handling: refresh before reading, rotated refresh
//! tokens kept, one retry after a 401, and disconnecting forgetting it all.

use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use deino_domain::{FakeClock, ServiceId};
use deino_net::{FakeNetClient, FakeReply, FakeServicePolicy, ServicePolicy, Status};
use deino_secrets::{InMemorySecretStore, SecretName};

use super::*;
use crate::calendar::MicrosoftCalendar;
use crate::fake::FakeBrowser;
use crate::oauth::{MICROSOFT, OAuthClient, OAuthProvider};

/// Credentials that are always set up (or never, with `None`).
struct FixedCredentials(Option<&'static str>);

#[async_trait]
impl Credentials for FixedCredentials {
    async fn client(
        &self,
        _provider: CalendarProvider,
        oauth: &'static OAuthProvider,
    ) -> Result<OAuthClient, OAuthError> {
        let id = self.0.ok_or(OAuthError::NotConfigured(oauth.name))?;
        Ok(OAuthClient::new(&MICROSOFT, id.into(), None))
    }
}

struct Harness {
    connector: ConnectorSource,
    http: Arc<FakeNetClient>,
    secrets: Arc<InMemorySecretStore>,
    policy: Arc<FakeServicePolicy>,
    clock: Arc<FakeClock>,
}

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-10-09T09:00:00Z")
        .unwrap()
        .to_utc()
}

fn harness(client_id: Option<&'static str>) -> Harness {
    let (http, secrets) = (
        Arc::new(FakeNetClient::default()),
        Arc::new(InMemorySecretStore::default()),
    );
    let (policy, clock) = (
        Arc::new(FakeServicePolicy::default()),
        Arc::new(FakeClock::new(now())),
    );
    let connector = ConnectorSource::new(ConnectorParts {
        api: Arc::new(MicrosoftCalendar),
        http: http.clone(),
        credentials: Arc::new(FixedCredentials(client_id)),
        secrets: secrets.clone(),
        switch: policy.clone(),
        browser: Arc::new(FakeBrowser::new()),
        clock: clock.clone(),
    });
    Harness {
        connector,
        http,
        secrets,
        policy,
        clock,
    }
}

async fn connect(secrets: &InMemorySecretStore) {
    let token = SecretValue::parse("refresh-1").unwrap();
    secrets
        .set(SecretName::MicrosoftCalendarRefreshToken, token)
        .await
        .unwrap();
}

/// Scripts a token refresh and an empty Graph account.
fn script_refresh_and_empty_account(http: &FakeNetClient, refresh: &str) {
    http.reply(FakeReply::ok(format!(
        r#"{{"access_token":"access-x","refresh_token":"{refresh}","expires_in":3600}}"#
    )));
    http.reply(FakeReply::ok(r#"{"mail":"maya@contoso.com"}"#));
    http.reply(FakeReply::ok(r#"{"value":[]}"#));
}

fn window() -> SyncWindow {
    SyncWindow::around(now())
}

#[tokio::test]
async fn an_unconnected_account_has_no_access_and_reads_nothing() {
    let Harness {
        connector, http, ..
    } = harness(Some("client"));

    assert_eq!(connector.access().await, CalendarAccess::NotDetermined);
    assert!(matches!(
        connector.fetch(window()).await,
        Err(SourceError::AccessDenied)
    ));
    assert!(http.requests().is_empty());
}

#[tokio::test]
async fn a_read_refreshes_first_keeps_the_rotated_token_and_reuses_the_access_token() {
    let Harness {
        connector,
        http,
        secrets,
        clock,
        ..
    } = harness(Some("client"));
    connect(&secrets).await;
    script_refresh_and_empty_account(&http, "refresh-2");
    http.reply(FakeReply::ok(r#"{"mail":"maya@contoso.com"}"#));
    http.reply(FakeReply::ok(r#"{"value":[]}"#));

    connector.fetch(window()).await.unwrap();
    clock.advance(TimeDelta::minutes(30));
    let snapshot = connector.fetch(window()).await.unwrap();

    assert_eq!(snapshot.accounts[0].name, "maya@contoso.com");
    let hosts: Vec<String> = http
        .requests()
        .iter()
        .map(|r| r.url.host_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        hosts
            .iter()
            .filter(|h| *h == "login.microsoftonline.com")
            .count(),
        1
    );
    let saved = secrets
        .get(SecretName::MicrosoftCalendarRefreshToken)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.expose(), "refresh-2");
    assert_eq!(connector.access().await, CalendarAccess::Granted);
}

#[tokio::test]
async fn a_401_refreshes_once_and_retries() {
    let Harness {
        connector,
        http,
        secrets,
        ..
    } = harness(Some("client"));
    connect(&secrets).await;
    http.reply(FakeReply::ok(
        r#"{"access_token":"stale","expires_in":3600}"#,
    ));
    http.reply(FakeReply::status(Status::new(401)));
    script_refresh_and_empty_account(&http, "refresh-3");

    let snapshot = connector.fetch(window()).await;

    assert_eq!(snapshot.unwrap().accounts.len(), 1);
    let last = http.requests().pop().unwrap();
    assert_eq!(last.headers.get("authorization"), Some("Bearer access-x"));
}

#[tokio::test]
async fn a_revoked_sign_in_reports_lost_access() {
    let Harness {
        connector,
        http,
        secrets,
        ..
    } = harness(Some("client"));
    connect(&secrets).await;
    http.reply(FakeReply::ok(r#"{"error":"invalid_grant"}"#).with_status(Status::new(400)));

    let outcome = connector.fetch(window()).await;

    assert!(matches!(outcome, Err(SourceError::AccessDenied)));
}

#[tokio::test]
async fn connecting_without_a_client_id_changes_nothing() {
    let Harness {
        connector, policy, ..
    } = harness(None);

    let outcome = connector.request_access().await;

    let message = outcome.unwrap_err().to_string();
    assert_eq!(message, "could not read your calendars");
    assert!(!policy.grant(ServiceId::MicrosoftGraph).await.enabled);
}

#[tokio::test]
async fn disconnecting_forgets_the_token_and_switches_the_service_off() {
    let Harness {
        connector,
        secrets,
        policy,
        ..
    } = harness(Some("client"));
    connect(&secrets).await;
    policy
        .set_enabled(ServiceId::MicrosoftGraph, true)
        .await
        .unwrap();

    connector.disconnect().await.unwrap();

    assert_eq!(connector.access().await, CalendarAccess::NotDetermined);
    assert!(!policy.grant(ServiceId::MicrosoftGraph).await.enabled);
}
