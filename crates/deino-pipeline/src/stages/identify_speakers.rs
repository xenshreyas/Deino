//! `identify_speakers` (order 35, CAP-04): names the diarized speakers from
//! voice profiles and the calendar, and leaves "Is this Alex?" suggestions.
//!
//! The rules are `deino_core::diarization::identify` behind the
//! [`SpeakerIdentifier`] port: the user's microphone is "Me"; unnamed voices
//! are matched against everyone's exemplars (auto-assign with a margin and
//! mutual best, attendees preferred, the rest by elimination); speakers the
//! user named are never touched. Running it again replaces only its own
//! suggestions, so a retry or a re-run is safe.

use std::sync::Arc;

use async_trait::async_trait;
use deino_core::diarization::{Identified, SpeakerIdentifier};
use deino_domain::{MeetingId, StageId};

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "identify_speakers";
/// Where it runs: after `diarize` (30), before vocabulary fixes and notes
/// (§4.2), so notes are written with names.
const ORDER: u16 = 35;
/// Shown when Settings turn identification off.
const DISABLED: &str = "Naming speakers by voice is off in Settings.";
/// Shown when the meeting has no speakers to name.
const NO_SPEAKERS: &str = "The meeting has no speakers to name.";

/// Names a meeting's speakers.
pub struct IdentifySpeakersStage {
    identifier: Arc<dyn SpeakerIdentifier>,
}

impl IdentifySpeakersStage {
    /// The stage over the identifier.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            identifier: Arc::clone(&ports.identifier),
        }
    }
}

#[async_trait]
impl PipelineStage for IdentifySpeakersStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        let identified = tokio::select! {
            biased;
            () = ctx.cancel.cancelled() => return Err(StageError::Cancelled),
            identified = self.identifier.identify(&meeting) => identified,
        };
        match identified {
            Ok(Identified::Done { .. }) => Ok(StageOutcome::Done),
            Ok(Identified::Disabled) => Ok(StageOutcome::Skipped(DISABLED)),
            Ok(Identified::NoSpeakers) => Ok(StageOutcome::Skipped(NO_SPEAKERS)),
            Err(error) => Err(StageError::retryable_from(&error)),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedIdentifications;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};

    use async_trait::async_trait;
    use deino_core::diarization::{Identified, IdentifyError, SpeakerIdentifier};
    use deino_domain::MeetingId;

    /// A [`SpeakerIdentifier`] for tests: records each meeting and names
    /// nobody.
    #[derive(Debug, Default)]
    pub struct RecordedIdentifications {
        meetings: Mutex<Vec<MeetingId>>,
    }

    impl RecordedIdentifications {
        /// Every meeting asked about, oldest first.
        pub fn meetings(&self) -> Vec<MeetingId> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl SpeakerIdentifier for RecordedIdentifications {
        async fn identify(&self, meeting: &MeetingId) -> Result<Identified, IdentifyError> {
            self.meetings
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(meeting.clone());
            Ok(Identified::Done {
                named: 0,
                suggested: 0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use deino_core::diarization::IdentifyError;
    use deino_db::DbError;
    use deino_domain::FakeClock;
    use deino_testkit::at;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::RecordedProgress;

    /// Answers every meeting with one scripted result.
    struct Scripted(fn() -> Result<Identified, IdentifyError>);

    #[async_trait]
    impl SpeakerIdentifier for Scripted {
        async fn identify(&self, _meeting: &MeetingId) -> Result<Identified, IdentifyError> {
            (self.0)()
        }
    }

    fn ctx() -> StageCtx {
        StageCtx {
            attempt: 1,
            cancel: CancellationToken::new(),
            clock: Arc::new(FakeClock::new(at("2026-10-11T09:00:00Z"))),
            events: Arc::new(RecordedProgress::default()),
        }
    }

    async fn run(
        script: fn() -> Result<Identified, IdentifyError>,
    ) -> Result<StageOutcome, StageError> {
        let stage = IdentifySpeakersStage::new(&StagePorts {
            identifier: Arc::new(Scripted(script)),
            ..StagePorts::fakes()
        });
        assert_eq!((stage.id(), stage.order()), (StageId::new(ID), ORDER));
        stage.run(&ctx(), MeetingId::new("meeting-a")).await
    }

    #[tokio::test]
    async fn outcomes_map_to_done_or_a_reason() {
        let done = || {
            Ok(Identified::Done {
                named: 2,
                suggested: 1,
            })
        };
        assert_eq!(run(done).await, Ok(StageOutcome::Done));
        assert_eq!(
            run(|| Ok(Identified::Disabled)).await,
            Ok(StageOutcome::Skipped(DISABLED))
        );
        assert_eq!(
            run(|| Ok(Identified::NoSpeakers)).await,
            Ok(StageOutcome::Skipped(NO_SPEAKERS))
        );
    }

    #[tokio::test]
    async fn a_storage_failure_is_retried() {
        let failing = || {
            Err(IdentifyError::Storage(DbError::Query(
                sqlx::Error::PoolClosed,
            )))
        };
        assert!(run(failing).await.unwrap_err().is_retryable());
    }
}
