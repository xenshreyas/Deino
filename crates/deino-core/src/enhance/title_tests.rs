//! `AutoTitler` over the seeded fixtures: only generic titles change, a
//! calendar title wins without an LLM call, notes beat the transcript.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{FakeClock, MeetingId};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::at;
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};

use super::*;
use crate::meetings::{MeetingRepo, SqliteMeetingRepo};

struct Harness {
    titler: AutoTitler,
    database: Database,
    mock: Arc<MockProvider>,
}

async fn harness(mock: MockProvider) -> Harness {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    let mock = Arc::new(mock);
    let titler = AutoTitler::new(
        MaterialSources::sqlite(&database),
        Arc::new(SqliteEnhancedNotesRepo::new(database.clone())),
        Arc::new(mock_gateway(mock.clone()).gateway),
        Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
    );
    Harness {
        titler,
        database,
        mock,
    }
}

fn fixture(kind: MeetingKind) -> MeetingId {
    Fixtures::load().unwrap().meeting(kind).unwrap().id.clone()
}

async fn retitle(database: &Database, meeting: &MeetingId, title: &str, unlink: bool) {
    sqlx::query("UPDATE meetings SET title = ?1 WHERE id = ?2")
        .bind(title)
        .bind(meeting.as_str())
        .execute(database.writer())
        .await
        .unwrap();
    if unlink {
        sqlx::query("UPDATE meetings SET calendar_event_id = NULL WHERE id = ?1")
            .bind(meeting.as_str())
            .execute(database.writer())
            .await
            .unwrap();
    }
}

async fn title_of(database: &Database, meeting: &MeetingId) -> String {
    SqliteMeetingRepo::new(database.clone())
        .find(meeting)
        .await
        .unwrap()
        .unwrap()
        .title
}

#[tokio::test]
async fn a_generic_title_is_replaced_from_the_enhanced_notes() {
    let harness =
        harness(MockProvider::new().reply(Route::Title, "\"Maya off on-call, Graph renewals.\""))
            .await;
    let meeting = fixture(MeetingKind::OneOnOne);
    retitle(
        &harness.database,
        &meeting,
        "Meeting 2026-10-07_15-00-00",
        true,
    )
    .await;
    let doc = deino_doc::from_markdown("### Updates\n\n- Maya is off on-call").unwrap();
    SqliteEnhancedNotesRepo::new(harness.database.clone())
        .insert(&NewVersion {
            meeting_id: meeting.clone(),
            markdown: "### Updates\n\n- Maya is off on-call\n".into(),
            doc,
            template_id: None,
            model: "builtin:mock-local".into(),
            basis: None,
            is_user_edited: false,
            at: at("2026-10-07T16:00:00Z"),
        })
        .await
        .unwrap();

    let run = harness.titler.run(&meeting).await.unwrap();

    assert_eq!(
        run,
        TitleRun::Renamed("Maya off on-call, Graph renewals".into())
    );
    assert_eq!(
        title_of(&harness.database, &meeting).await,
        "Maya off on-call, Graph renewals"
    );
    let prompt = harness.mock.requests()[0].messages[0].text();
    assert!(prompt.contains("<notes>") && prompt.contains("Maya is off on-call"));
}

#[tokio::test]
async fn a_generic_title_without_notes_is_named_from_the_transcript() {
    let harness =
        harness(MockProvider::new().reply(Route::Title, "Release cut and offline list")).await;
    let meeting = fixture(MeetingKind::Standup);
    retitle(&harness.database, &meeting, "New note", true).await;

    harness.titler.run(&meeting).await.unwrap();

    assert!(
        harness.mock.requests()[0].messages[0]
            .text()
            .contains("<transcript>")
    );
    assert_eq!(
        title_of(&harness.database, &meeting).await,
        "Release cut and offline list"
    );
}

#[tokio::test]
async fn a_calendar_title_wins_without_asking_the_llm() {
    let harness = harness(MockProvider::new()).await;
    let meeting = fixture(MeetingKind::Standup);
    let calendar_title = Fixtures::load()
        .unwrap()
        .meeting(MeetingKind::Standup)
        .unwrap()
        .calendar_event
        .title
        .clone();
    retitle(&harness.database, &meeting, "New note", false).await;

    let run = harness.titler.run(&meeting).await.unwrap();

    assert_eq!(run, TitleRun::Renamed(calendar_title));
    assert!(harness.mock.requests().is_empty());
}

#[tokio::test]
async fn a_named_meeting_keeps_its_title() {
    let harness = harness(MockProvider::new()).await;

    let run = harness
        .titler
        .run(&fixture(MeetingKind::Planning))
        .await
        .unwrap();

    assert_eq!(run, TitleRun::Skipped(ALREADY_NAMED));
    assert!(harness.mock.requests().is_empty());
}
