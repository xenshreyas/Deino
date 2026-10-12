//! The briefs tests' world: the fixture library seeded into SQLite, a clock
//! at 06:30 Pacific on Monday 12 October 2026, the mock gateway, and
//! upcoming events with the fixtures' people or with strangers.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use deino_db::Database;
use deino_domain::{CalendarEventId, Clock, Email, FakeClock, FixedZone, LocalZone, Rsvp};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::fixture::{USER_EMAIL, fixture_uuid};
use deino_testkit::{at, seed_fixtures, test_db};

use super::{
    BriefDeps, BriefScheduler, BriefService, BriefSources, BriefSwitch, GatewayBriefWriter,
    SchedulerDeps, SqliteBriefRepo, UpcomingEvents,
};
use crate::calendar::{
    CalendarError, CalendarEvent, CalendarEventRepo, EventAttendee, SqliteCalendarRepo,
};
use crate::meetings::SqliteMeetingRepo;
use crate::people::{MeRepo, PersonRepo, SqlitePeopleRepo};
use crate::redaction::TextRedactor;
use crate::settings::BriefSettings;

/// The tests' "now": 06:30 in the fixtures' zone (UTC-7).
pub const NOW: &str = "2026-10-12T13:30:00Z";
/// Marcus Webb of Brightkite Logistics, met in the sales discovery fixture.
pub const MARCUS: &str = "marcus.webb@brightkite-logistics.com";
/// Someone the user has never met.
pub const STRANGER: &str = "ines@newco.example";

/// The fixtures' zone, Pacific daylight time.
pub fn pacific() -> Arc<dyn LocalZone> {
    Arc::new(FixedZone::hours(-7).unwrap())
}

/// A database holding the eight fixture meetings, the fixtures' user marked
/// as the Me person (as the profile does in the app).
pub async fn seeded() -> Database {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let people = SqlitePeopleRepo::new(database.clone());
    let user = people
        .find_by_email(&Email::parse(USER_EMAIL).unwrap())
        .await
        .unwrap()
        .unwrap();
    people.make_me(&user.id, at(NOW)).await.unwrap();
    database
}

/// An external 30-minute meeting with the user and `guest`, starting
/// `starts_in` after [`NOW`], stored in the fixture calendar account.
pub async fn upcoming(database: &Database, guest: &str, starts_in: TimeDelta) -> CalendarEvent {
    let clock = FakeClock::new(at(NOW));
    let start = at(NOW) + starts_in;
    let event = CalendarEvent {
        id: CalendarEventId::generate(&clock),
        account_id: fixture_uuid("calendar-account").parse().unwrap(),
        calendar_id: None,
        provider_event_id: format!("upcoming-{guest}-{}", start.timestamp()),
        ical_uid: None,
        series_id: None,
        title: "Brightkite follow-up".to_owned(),
        description: "Agenda: pilot scope and pricing.".to_owned(),
        start_at: start,
        end_at: start + TimeDelta::minutes(30),
        is_all_day: false,
        conference_url: None,
        platform: None,
        organizer_email: Some(USER_EMAIL.to_owned()),
        attendees: vec![attendee(USER_EMAIL), attendee(guest)],
        my_rsvp: Some(Rsvp::Accepted),
        is_external: true,
        record_override: None,
        created_at: at(NOW),
        updated_at: at(NOW),
    };
    SqliteCalendarRepo::new(database.clone())
        .upsert_event(&event)
        .await
        .unwrap();
    event
}

fn attendee(email: &str) -> EventAttendee {
    EventAttendee {
        email: Some(email.to_owned()),
        name: None,
        rsvp: Some(Rsvp::Accepted),
        is_organizer: email == USER_EMAIL,
    }
}

/// A known-good answer citing the first meeting and the invitation.
pub const GOOD_ANSWER: &str = r#"{"insufficient": false, "bullets": [
    {"text": "Brightkite's DispatchPro licence ends on 31 January 2027, a hard deadline.", "sources": ["m1"]},
    {"text": "The agenda covers pilot scope and pricing.", "sources": ["e1", "m1"]},
    {"text": "An invented claim.", "sources": ["m9"]}
]}"#;

/// The service and what tests inspect.
pub struct Briefs {
    /// The service under test.
    pub service: Arc<BriefService>,
    /// The mock behind the gateway.
    pub llm: Arc<MockProvider>,
}

/// A service over `database` whose model answers [`GOOD_ANSWER`], hiding
/// secrets with `redactor` when one is given.
pub fn briefs(database: &Database, redactor: Option<Arc<dyn TextRedactor>>) -> Briefs {
    answering(database, redactor, GOOD_ANSWER)
}

/// A service over `database` whose model always answers `answer`.
pub fn answering(
    database: &Database,
    redactor: Option<Arc<dyn TextRedactor>>,
    answer: &'static str,
) -> Briefs {
    let llm = Arc::new(MockProvider::new().answer_with(Route::Brief, move |_| answer.to_owned()));
    let gateway = Arc::new(mock_gateway(llm.clone()).gateway);
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at(NOW)));
    let mut sources = BriefSources::sqlite(database, pacific());
    if let Some(redactor) = redactor {
        sources = sources.with_redactor(redactor);
    }
    let service = BriefService::new(BriefDeps {
        store: Arc::new(SqliteBriefRepo::new(database.clone())),
        events: Arc::new(SqliteCalendarRepo::new(database.clone())),
        meetings: Arc::new(SqliteMeetingRepo::new(database.clone())),
        sources,
        writer: Arc::new(GatewayBriefWriter::new(gateway)),
        clock,
    });
    Briefs {
        service: Arc::new(service),
        llm,
    }
}

/// A redactor that hides one word and remembers what it saw.
#[derive(Debug, Default)]
pub struct WordRedactor {
    /// Every text it was given.
    pub seen: Mutex<Vec<String>>,
}

/// The word [`WordRedactor`] hides.
pub const SECRET: &str = "dispatchpro";

#[async_trait]
impl TextRedactor for WordRedactor {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        self.seen.lock().unwrap().extend(texts.iter().cloned());
        texts
            .into_iter()
            .map(|text| text.replace(SECRET, "[redacted]"))
            .collect()
    }
}

/// `at(NOW)` plus `minutes`.
pub fn later(minutes: i64) -> DateTime<Utc> {
    at(NOW) + TimeDelta::minutes(minutes)
}

/// The stored events, as the calendar would list them.
pub struct StoredEvents(pub SqliteCalendarRepo);

#[async_trait]
impl UpcomingEvents for StoredEvents {
    async fn starting(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>, CalendarError> {
        Ok(self.0.events_between(from, to).await?)
    }
}

/// Fixed brief settings.
pub struct Switch(pub BriefSettings);

#[async_trait]
impl BriefSwitch for Switch {
    async fn brief_settings(&self) -> BriefSettings {
        self.0
    }
}

/// The job over `briefs`, the stored events and `clock`, with `settings`.
pub fn scheduler(
    database: &Database,
    briefs: &Briefs,
    clock: Arc<FakeClock>,
    settings: BriefSettings,
) -> BriefScheduler {
    BriefScheduler::new(SchedulerDeps {
        service: briefs.service.clone(),
        events: Arc::new(StoredEvents(SqliteCalendarRepo::new(database.clone()))),
        switch: Arc::new(Switch(settings)),
        clock,
        zone: pacific(),
    })
}
