//! The service over real SQLite repositories and fake ports.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use deino_db::{Database, test_db};
use deino_domain::{FakeClock, IntegrationProvider};
use deino_net::{EgressLog, FakeNetClient, FakeServicePolicy, Net, NetParts};
use deino_secrets::InMemoryCredentialStore;

use super::*;
use crate::{
    Action, AuthSpec, CredentialVault, DeliveryRequest, Egress, FakeBrowser, FakeOAuthApps,
    FakeSleeper, IdempotencyKey, Integration, MessageDraft, NetEgress, ScriptedIntegration,
    SqliteConnectionRepo, SqliteDeliveryRepo, TokenScheme, TokenSpec,
};

pub(super) struct Harness {
    pub(super) service: IntegrationService,
    pub(super) store: Arc<InMemoryCredentialStore>,
    pub(super) apps: Arc<FakeOAuthApps>,
    pub(super) browser: Arc<FakeBrowser>,
    pub(super) clock: Arc<FakeClock>,
    pub(super) sleeper: Arc<FakeSleeper>,
    pub(super) database: Database,
}

/// The service with `integrations`, sending through `egress`.
pub(super) async fn harness(
    integrations: Vec<Arc<dyn Integration>>,
    egress: Arc<dyn Egress>,
) -> Harness {
    let database = test_db().await;
    let clock = Arc::new(FakeClock::new(
        Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap(),
    ));
    let sleeper = Arc::new(FakeSleeper::new(clock.clone()));
    let store = Arc::new(InMemoryCredentialStore::default());
    let apps = Arc::new(FakeOAuthApps::default());
    let browser = Arc::new(FakeBrowser::default());
    let service = IntegrationService::new(IntegrationPorts {
        registry: IntegrationRegistry::new(integrations),
        connections: Arc::new(SqliteConnectionRepo::new(database.clone())),
        deliveries: Arc::new(SqliteDeliveryRepo::new(database.clone())),
        vault: CredentialVault::new(store.clone()),
        apps: apps.clone(),
        egress,
        browser: browser.clone(),
        clock: clock.clone(),
        sleeper: sleeper.clone(),
    });
    Harness {
        service,
        store,
        apps,
        browser,
        clock,
        sleeper,
        database,
    }
}

/// `deino-net` over a scripted client: real scoping, policy and egress log.
pub(super) struct ScriptedNet {
    pub(super) fake: Arc<FakeNetClient>,
    pub(super) policy: Arc<FakeServicePolicy>,
    pub(super) egress_log: Arc<EgressLog>,
    pub(super) egress: Arc<NetEgress>,
}

pub(super) fn scripted_net() -> ScriptedNet {
    let fake = Arc::new(FakeNetClient::default());
    let policy = Arc::new(FakeServicePolicy::default());
    let egress_log = Arc::new(EgressLog::default());
    let net = Net::new(NetParts {
        client: fake.clone(),
        policy: policy.clone(),
        egress: egress_log.clone(),
        clock: Arc::new(FakeClock::new(
            Utc.with_ymd_and_hms(2026, 10, 11, 9, 0, 0).unwrap(),
        )),
    });
    let egress = Arc::new(NetEgress::new(net, policy.clone(), policy.clone()));
    ScriptedNet {
        fake,
        policy,
        egress_log,
        egress,
    }
}

/// A Linear-like provider: a bare API key, and `400 RATELIMITED`.
pub(super) fn linear() -> Arc<dyn Integration> {
    let token = TokenSpec {
        label: "API key",
        username_label: None,
        scheme: TokenScheme::Bare,
        help_url: "https://linear.app/settings/api".into(),
    };
    Arc::new(ScriptedIntegration::new(
        IntegrationProvider::Linear,
        vec![AuthSpec::Token(token)],
        "https://api.linear.app",
    ))
}

/// A message delivery with `key`.
pub(super) fn message(connection: deino_domain::ConnectionId, key: &str) -> DeliveryRequest {
    DeliveryRequest {
        connection,
        action: Action::PostMessage(MessageDraft {
            target: "C1".into(),
            text: "Notes".into(),
            replies: Vec::new(),
        }),
        idempotency_key: IdempotencyKey::parse(key).unwrap(),
        meeting: None,
    }
}

/// Every text value stored in the integration tables, to prove no secret
/// is among them.
pub(super) async fn stored_text(database: &Database) -> String {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT coalesce(account_id,'') || account_label || coalesce(api_base_url,'') FROM integration_connections
         UNION ALL SELECT scope FROM integration_connection_scopes
         UNION ALL SELECT idempotency_key || coalesce(external_id,'') || coalesce(error_message,'') FROM integration_deliveries",
    )
    .fetch_all(database.reader())
    .await
    .unwrap();
    rows.into_iter().map(|(text,)| text).collect()
}
