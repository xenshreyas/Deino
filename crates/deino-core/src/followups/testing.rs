//! The follow-up tests' world: the fixture library seeded into SQLite with
//! the fixtures' user as the Me person and as the profile's address, the
//! mock gateway, and the recorded mail app and clipboard.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use deino_db::Database;
use deino_domain::{Email, FakeClock};
use deino_llm::followup::{FollowUpDecision, FollowUpDraftAnswer, FollowUpVerdict};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::fixture::USER_EMAIL;
use deino_testkit::{at, seed_fixtures, test_db};

use super::{
    FollowUpDeps, FollowUpGenerator, FollowUpPorts, FollowUpService, GatewayFollowUpModel,
    SqliteFollowUpRepo,
};
use crate::enhance::MaterialSources;
use crate::export::{RecordedClipboard, RecordedOpener};
use crate::insights::FixedDomains;
use crate::people::{MeRepo, PersonRepo, SqlitePeopleRepo};
use crate::profile::InternalDomains;
use crate::redaction::TextRedactor;

/// The tests' "now": the evening after the sales discovery call.
pub const NOW: &str = "2026-09-30T18:00:00Z";
/// The sales discovery fixture: the user, two colleagues and two guests.
pub const SALES: &str = "meeting-fixture-sales-discovery";
/// The standup fixture: the user and four colleagues.
pub const STANDUP: &str = "meeting-fixture-standup";
/// The word [`WordRedactor`] hides; the discovery notes mention it.
pub const SECRET: &str = "dispatchpro";

/// A database holding the eight fixture meetings, the fixtures' user marked
/// as the Me person.
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

/// A `followup.email` reply with `verdict`.
pub fn decision(verdict: FollowUpVerdict) -> String {
    let decision = FollowUpDecision {
        reason: "Next steps were agreed.".into(),
        verdict,
    };
    serde_json::to_string(&decision).unwrap()
}

/// A `followup.email` draft reply.
pub fn draft() -> String {
    let draft = FollowUpDraftAnswer {
        subject: "Re: Brightkite pilot: next steps".into(),
        body: "Hi Marcus and Hannah,\n\nThanks for today. I'll send the pilot proposal by [proposal date].\n\nShreyas".into(),
    };
    serde_json::to_string(&draft).unwrap()
}

/// The generator and service over `database` and one mock provider.
pub struct World {
    pub generator: FollowUpGenerator,
    pub service: FollowUpService,
    pub llm: Arc<MockProvider>,
    pub opener: Arc<RecordedOpener>,
    pub clipboard: Arc<RecordedClipboard>,
    pub clock: Arc<FakeClock>,
}

/// The world over `database`, the model answering with `llm`, the
/// transcript and notes redacted by `redactor` when given.
pub fn world(
    database: &Database,
    llm: MockProvider,
    redactor: Option<Arc<dyn TextRedactor>>,
) -> World {
    let llm = Arc::new(llm);
    let gateway = Arc::new(mock_gateway(llm.clone()).gateway);
    let clock = Arc::new(FakeClock::new(at(NOW)));
    let store = Arc::new(SqliteFollowUpRepo::new(database.clone()));
    let mut sources = MaterialSources::sqlite(database);
    if let Some(redactor) = redactor {
        sources = sources.with_redactor(redactor);
    }
    let domains = InternalDomains::from_emails(&[Email::parse(USER_EMAIL).unwrap()]);
    let generator = FollowUpGenerator::new(FollowUpDeps {
        store: store.clone(),
        participants: store.clone(),
        sources,
        domains: Arc::new(FixedDomains(domains)),
        model: Arc::new(GatewayFollowUpModel::new(gateway)),
        clock: clock.clone(),
    });
    let opener = Arc::new(RecordedOpener::default());
    let clipboard = Arc::new(RecordedClipboard::default());
    let service = FollowUpService::new(FollowUpPorts {
        store,
        opener: opener.clone(),
        clipboard: clipboard.clone(),
        clock: clock.clone(),
    });
    World {
        generator,
        service,
        llm,
        opener,
        clipboard,
        clock,
    }
}

/// A model that decides `send` and drafts [`draft`].
pub fn sending() -> MockProvider {
    MockProvider::new()
        .reply(Route::FollowupEmail, &decision(FollowUpVerdict::Send))
        .reply(Route::FollowupEmail, &draft())
}

/// Hides [`SECRET`] and records what it saw.
#[derive(Default)]
pub struct WordRedactor {
    /// Every text it was given.
    pub seen: Mutex<Vec<String>>,
}

#[async_trait]
impl TextRedactor for WordRedactor {
    async fn redact(&self, texts: Vec<String>) -> Vec<String> {
        self.seen.lock().unwrap().extend(texts.iter().cloned());
        texts
            .into_iter()
            .map(|text| text.to_lowercase().replace(SECRET, "[redacted]"))
            .collect()
    }
}
