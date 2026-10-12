//! The webhooks' test harness: the service over a real SQLite database,
//! a fake clock, a scripted HTTP client, an in-memory Keychain, and one
//! fixed meeting.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use deino_db::{Database, DbError, test_db};
use deino_domain::{ActionItemId, Clock, FakeClock, MeetingId, Millis};
use deino_export::{ActionItemExport, MeetingExport, Participant, Utterance};
use deino_net::{FakeNetClient, FakeServicePolicy};
use deino_secrets::InMemoryCredentialStore;

use super::{
    CreatedWebhook, SqliteDeliveryRepo, SqliteWebhookRepo, SubjectItem, SubjectItems, WebhookEvent,
    WebhookInput, WebhookPorts, WebhookSecrets, WebhookService,
};
use crate::automations::{MeetingSource, PayloadFields};
use crate::export::ExportError;
use crate::testing::lock;

/// The fixed meeting's ID.
pub(super) const MEETING: &str = "meeting-acme";

/// When every test starts.
pub(super) fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 12, 9, 0, 0).unwrap()
}

/// The meeting every payload reads.
pub(super) struct FixedMeeting;

#[async_trait]
impl MeetingSource for FixedMeeting {
    async fn export(&self, meeting: &MeetingId) -> Result<MeetingExport, ExportError> {
        if meeting.as_str() != MEETING {
            return Err(ExportError::NotFound(meeting.clone()));
        }
        Ok(MeetingExport {
            id: meeting.clone(),
            title: "Acme sync".into(),
            started_at: DateTime::parse_from_rfc3339("2026-10-12T09:00:00-07:00").unwrap(),
            duration: Some(deino_domain::Seconds(1_800)),
            attendees: vec![Participant {
                name: "Maya Chen".into(),
                email: Some("maya@acme.test".into()),
                is_me: false,
            }],
            tags: vec!["Sales".into()],
            folders: vec!["Clients".into()],
            notes: Some("## Summary\n\nAcme renews.".into()),
            my_notes: None,
            action_items: vec![ActionItemExport {
                title: "Send the quote".into(),
                description: String::new(),
                assignee: Some("Maya Chen".into()),
                due: None,
                done: false,
            }],
            insights: Vec::new(),
            transcript: vec![Utterance {
                speaker: "Maya Chen".into(),
                is_me: false,
                start: Millis(0),
                end: Millis(900),
                text: "We renew.".into(),
            }],
        })
    }

    async fn audio_path(&self, _meeting: &MeetingId) -> Result<Option<String>, ExportError> {
        Ok(Some("/recordings/meeting-acme/audio.m4a".into()))
    }
}

/// Action items by ID, all on the fixed meeting.
#[derive(Default)]
pub(super) struct FakeItems {
    items: Mutex<HashMap<ActionItemId, SubjectItem>>,
}

impl FakeItems {
    /// Adds `item` on the fixed meeting.
    pub(super) fn add(&self, item: SubjectItem) {
        lock(&self.items).insert(item.id, item);
    }
}

#[async_trait]
impl SubjectItems for FakeItems {
    async fn meeting_of(&self, item: &ActionItemId) -> Result<Option<MeetingId>, DbError> {
        Ok(lock(&self.items)
            .contains_key(item)
            .then(|| MeetingId::new(MEETING)))
    }

    async fn subject(&self, item: &ActionItemId) -> Result<Option<SubjectItem>, DbError> {
        Ok(lock(&self.items).get(item).cloned())
    }
}

/// The service and every fake behind it.
pub(super) struct Harness {
    pub(super) database: Database,
    pub(super) clock: Arc<FakeClock>,
    pub(super) client: Arc<FakeNetClient>,
    pub(super) policy: Arc<FakeServicePolicy>,
    pub(super) items: Arc<FakeItems>,
    pub(super) service: WebhookService,
}

impl Harness {
    /// A database with the fixed meeting (finished), and no endpoints.
    pub(super) async fn new() -> Self {
        let database = test_db().await;
        deino_testkit::seed::meeting(&database, MEETING, "Acme sync", start()).await;
        let clock = Arc::new(FakeClock::new(start()));
        let client = Arc::new(FakeNetClient::default());
        let policy = Arc::new(FakeServicePolicy::default());
        let items = Arc::new(FakeItems::default());
        let service = WebhookService::new(WebhookPorts {
            webhooks: Arc::new(SqliteWebhookRepo::new(database.clone())),
            deliveries: Arc::new(SqliteDeliveryRepo::new(database.clone())),
            secrets: WebhookSecrets::new(Arc::new(InMemoryCredentialStore::default())),
            meetings: Arc::new(FixedMeeting),
            items: items.clone(),
            client: client.clone(),
            switch: policy.clone(),
            clock: clock.clone(),
        });
        Self {
            database,
            clock,
            client,
            policy,
            items,
            service,
        }
    }

    /// Adds an endpoint for `events` with the default fields.
    pub(super) async fn add(&self, events: &[WebhookEvent]) -> CreatedWebhook {
        self.service
            .create(input(events, PayloadFields::default()))
            .await
            .unwrap()
    }

    /// Moves the clock forward to `at`.
    pub(super) fn advance_to(&self, at: DateTime<Utc>) {
        self.clock.advance(at - self.clock.now());
    }
}

/// An enabled endpoint's input.
pub(super) fn input(events: &[WebhookEvent], fields: PayloadFields) -> WebhookInput {
    WebhookInput {
        url: "https://hooks.zapier.com/hooks/catch/1/abc/".into(),
        label: "Zapier".into(),
        events: events.to_vec(),
        fields,
        enabled: true,
    }
}
