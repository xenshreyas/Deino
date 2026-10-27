//! `link_speakers`: runs between naming the speakers and writing the notes,
//! asks the updater to link them, retries a busy database and stops when
//! cancelled.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use deino_core::entities::{EntityError, EntityUpdate, EntityUpdater};
use deino_db::DbError;
use deino_domain::{FakeClock, MeetingId, StageId};
use deino_testkit::at;
use tokio_util::sync::CancellationToken;

use super::LinkSpeakersStage;
use crate::registry::stages;
use crate::{PipelineStage, RecordedProgress, StageCtx, StageError, StageOutcome, StagePorts};

fn ctx(cancel: CancellationToken) -> StageCtx {
    StageCtx {
        attempt: 1,
        cancel,
        clock: Arc::new(FakeClock::new(at("2026-10-27T12:00:00Z"))),
        events: Arc::new(RecordedProgress::default()),
    }
}

fn stage(updater: Arc<dyn EntityUpdater>) -> LinkSpeakersStage {
    let mut ports = StagePorts::fakes();
    ports.entities = updater;
    LinkSpeakersStage::new(&ports)
}

/// Links once per call; summaries are never asked for.
#[derive(Default)]
struct Counting(AtomicUsize);

#[async_trait::async_trait]
impl EntityUpdater for Counting {
    async fn update_entities(&self, _meeting: &MeetingId) -> Result<EntityUpdate, EntityError> {
        unreachable!("linking speakers never refreshes summaries")
    }

    async fn link_speakers(&self, _meeting: &MeetingId) -> Result<usize, EntityError> {
        Ok(self.0.fetch_add(1, Ordering::SeqCst) + 1)
    }
}

/// Fails with a busy database, or never answers.
enum Broken {
    Busy,
    Hang,
}

#[async_trait::async_trait]
impl EntityUpdater for Broken {
    async fn update_entities(&self, _meeting: &MeetingId) -> Result<EntityUpdate, EntityError> {
        unreachable!("linking speakers never refreshes summaries")
    }

    async fn link_speakers(&self, _meeting: &MeetingId) -> Result<usize, EntityError> {
        match self {
            Self::Busy => Err(EntityError::Storage(DbError::Query(
                sqlx::Error::PoolTimedOut,
            ))),
            Self::Hang => std::future::pending().await,
        }
    }
}

#[test]
fn it_runs_after_the_speakers_are_named_and_before_the_notes() {
    let ids: Vec<String> = stages(&StagePorts::fakes())
        .iter()
        .map(|stage| stage.id().to_string())
        .collect();
    let at = |id: &str| ids.iter().position(|known| known == id).unwrap();

    assert!(at("identify_speakers") < at("link_speakers"));
    assert!(at("link_speakers") < at("enhance_notes"));
    let stage = LinkSpeakersStage::new(&StagePorts::fakes());
    assert_eq!(
        (stage.id(), stage.order()),
        (StageId::new("link_speakers"), 47)
    );
}

#[tokio::test]
async fn it_links_the_named_speakers() {
    let updater = Arc::new(Counting::default());

    let outcome = stage(updater.clone())
        .run(&ctx(CancellationToken::new()), MeetingId::new("meeting-a"))
        .await;

    assert_eq!(outcome, Ok(StageOutcome::Done));
    assert_eq!(updater.0.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_busy_database_is_retried_and_cancelling_stops_it() {
    let busy = stage(Arc::new(Broken::Busy))
        .run(&ctx(CancellationToken::new()), MeetingId::new("meeting-a"))
        .await;
    let cancel = CancellationToken::new();
    cancel.cancel();
    let cancelled = stage(Arc::new(Broken::Hang))
        .run(&ctx(cancel), MeetingId::new("meeting-a"))
        .await;

    assert!(busy.unwrap_err().is_retryable());
    assert_eq!(cancelled, Err(StageError::Cancelled));
}
