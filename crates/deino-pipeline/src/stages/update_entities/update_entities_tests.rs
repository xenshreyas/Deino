//! `update_entities` over the real entity services, SQLite with the seeded
//! fixture meetings, and a scripted model (ORG-04's acceptance: summaries
//! refresh after a new meeting).

use std::sync::Arc;

use deino_core::entities::{
    EntityError, EntityPorts, EntitySummarizer, EntityUpdate, EntityUpdatePorts,
    EntityUpdateService, EntityUpdater,
};
use deino_core::people::{PersonRepo, SqlitePeopleRepo};
use deino_core::segments::SqliteSegmentRepo;
use deino_db::{Database, DbError, test_db};
use deino_domain::{Clock, Email, ErrorCode, FakeClock, MeetingId, StageId};
use deino_llm::{MockProvider, Route, mock_gateway};
use deino_testkit::at;
use deino_testkit::fixture::{Fixtures, MeetingKind, seed_fixtures};
use tokio_util::sync::CancellationToken;

use super::UpdateEntitiesStage;
use crate::{PipelineStage, RecordedProgress, StageCtx, StageError, StageOutcome, StagePorts};

const NOW: &str = "2026-10-11T12:00:00Z";

fn ctx(cancel: CancellationToken) -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel,
        clock: Arc::new(FakeClock::new(at(NOW))),
        events: Arc::new(RecordedProgress::default()),
    }
}

/// The seeded fixtures, with the user marked as "Me" as the profile does.
async fn seeded() -> Database {
    let database = test_db().await;
    seed_fixtures(&database).await.unwrap();
    sqlx::query("UPDATE people SET is_me = 1 WHERE email = 'shreyas@deino.dev'")
        .execute(database.writer())
        .await
        .unwrap();
    database
}

/// The stage over the real updater, answering `entity.summary` with `mock`.
fn stage(database: &Database, mock: MockProvider) -> UpdateEntitiesStage {
    let clock: Arc<dyn Clock> = Arc::new(FakeClock::new(at(NOW)));
    let entities = EntityPorts::sqlite(database, clock);
    let gateway = Arc::new(mock_gateway(Arc::new(mock)).gateway);
    let updater = EntityUpdateService::new(EntityUpdatePorts {
        summarizer: Arc::new(EntitySummarizer::new(entities.clone(), Some(gateway))),
        entities,
        speakers: Arc::new(SqliteSegmentRepo::new(database.clone())),
        logos: None,
    });
    let mut ports = StagePorts::fakes();
    ports.entities = Arc::new(updater);
    UpdateEntitiesStage::new(&ports)
}

#[tokio::test]
async fn a_new_meeting_refreshes_the_summaries_of_the_people_in_it() {
    let database = seeded().await;
    let fixture = Fixtures::load().unwrap();
    let meeting = &fixture.meeting(MeetingKind::OneOnOne).unwrap().id;
    let people = SqlitePeopleRepo::new(database.clone());
    let maya_email = Email::parse("maya@deino.dev").unwrap();
    let before = people.find_by_email(&maya_email).await.unwrap().unwrap();
    let mock = MockProvider::new()
        .reply(Route::EntitySummary, "Maya owns the webhook queue split.")
        .reply(Route::EntitySummary, "Deino is the user's own company.");

    let outcome = stage(&database, mock)
        .run(&ctx(CancellationToken::new()), meeting.clone())
        .await;

    assert_eq!(outcome, Ok(StageOutcome::Done));
    let after = people.find_by_email(&maya_email).await.unwrap().unwrap();
    assert_eq!(before.summary_md, None);
    assert_eq!(
        after.summary_md.as_deref(),
        Some("Maya owns the webhook queue split.")
    );
    assert_eq!(after.summary_updated_at, Some(at(NOW)));
}

#[tokio::test]
async fn a_model_with_nothing_scripted_is_a_permanent_failure() {
    let database = seeded().await;
    let meeting = Fixtures::load()
        .unwrap()
        .meeting(MeetingKind::OneOnOne)
        .unwrap()
        .id
        .clone();

    let outcome = stage(&database, MockProvider::new())
        .run(&ctx(CancellationToken::new()), meeting)
        .await;

    assert!(
        matches!(outcome, Err(StageError::Permanent(error)) if error.code == ErrorCode::Llm || error.code == ErrorCode::Unavailable)
    );
}

/// Fails once with a busy database, then hangs.
struct Busy;

#[async_trait::async_trait]
impl EntityUpdater for Busy {
    async fn update_entities(&self, _meeting: &MeetingId) -> Result<EntityUpdate, EntityError> {
        Err(EntityError::Storage(DbError::Query(
            sqlx::Error::PoolTimedOut,
        )))
    }
}

#[tokio::test]
async fn a_busy_database_is_retried() {
    let mut ports = StagePorts::fakes();
    ports.entities = Arc::new(Busy);

    let outcome = UpdateEntitiesStage::new(&ports)
        .run(&ctx(CancellationToken::new()), MeetingId::new("meeting-a"))
        .await;

    assert!(outcome.unwrap_err().is_retryable());
}

#[tokio::test]
async fn the_stage_is_update_entities_at_order_ninety_and_can_be_cancelled() {
    let stage = UpdateEntitiesStage::new(&StagePorts::fakes());
    let cancel = CancellationToken::new();
    cancel.cancel();

    assert_eq!(
        (stage.id(), stage.order()),
        (StageId::new("update_entities"), 90)
    );
    let mut ports = StagePorts::fakes();
    ports.entities = Arc::new(Hang);
    let outcome = UpdateEntitiesStage::new(&ports)
        .run(&ctx(cancel), MeetingId::new("m"))
        .await;
    assert_eq!(outcome, Err(StageError::Cancelled));
}

/// Never answers.
struct Hang;

#[async_trait::async_trait]
impl EntityUpdater for Hang {
    async fn update_entities(&self, _meeting: &MeetingId) -> Result<EntityUpdate, EntityError> {
        std::future::pending().await
    }
}
