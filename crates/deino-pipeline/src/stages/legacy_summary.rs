//! `legacy_summary` (order 50): the inherited summary generation as a stage.
//!
//! A temporary bridge until NOT-02's `enhance_notes` replaces it. The stage
//! reads the meeting's transcript, its speakers' names (CAP-04: a renamed
//! speaker reaches the notes on regeneration) and template and hands them
//! to the [`Summarizer`] port; the app implements the port over the legacy
//! summary service (`features/pipeline/legacy_summary.rs`), the one place
//! that calls it, so FND-09's gateway rewrite of the summary code touches
//! only that adapter.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use deino_core::meetings::MeetingLifecycleRepo;
use deino_core::segments::{Page, Segment, SegmentReader};
use deino_core::speakers::{ParticipantReader, ParticipantRow};
use deino_domain::{MeetingId, SpeakerId, StageId, TemplateId};
use tokio_util::sync::CancellationToken;

use crate::{PipelineStage, StageCtx, StageError, StageOutcome, StagePorts};

/// The stage's stored name.
const ID: &str = "legacy_summary";
/// Where it runs: the slot of notes generation (PARITY_PLAN §4.2).
const ORDER: u16 = 50;
/// Shown when automatic summaries are turned off.
const TURNED_OFF: &str = "Automatic summaries are turned off in Settings.";
/// Shown when there is nothing to summarise.
const NO_TRANSCRIPT: &str = "The meeting has no transcript to summarise.";
/// Seconds in a minute, for the `[MM:SS]` transcript timestamps.
const SECONDS_PER_MINUTE: u32 = 60;

/// What to summarise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryInput {
    /// The meeting the summary is for.
    pub meeting_id: MeetingId,
    /// The transcript, one `[MM:SS] Name: text` line per segment (`[MM:SS]
    /// text` when nobody is attributed), as the meeting page used to send it.
    pub transcript: String,
    /// The meeting's note template, or `None` for the default.
    pub template: Option<TemplateId>,
}

/// Generates and stores a meeting's summary.
#[async_trait]
pub trait Summarizer: Send + Sync {
    /// Whether summaries run after every meeting (`Settings::ai.auto_summary`).
    /// When off, the stage is skipped and the user generates one by hand.
    async fn is_automatic(&self) -> bool {
        true
    }

    /// Summarises `request`, stopping early when `cancel` fires.
    async fn generate(
        &self,
        request: SummaryInput,
        cancel: CancellationToken,
    ) -> Result<StageOutcome, StageError>;
}

/// Summarises the meeting through the legacy summary service.
pub struct LegacySummaryStage {
    segments: Arc<dyn SegmentReader>,
    participants: Arc<dyn ParticipantReader>,
    meetings: Arc<dyn MeetingLifecycleRepo>,
    summaries: Arc<dyn Summarizer>,
}

impl LegacySummaryStage {
    /// The stage over the ports it reads and the generator it calls.
    pub fn new(ports: &StagePorts) -> Self {
        Self {
            segments: Arc::clone(&ports.segments),
            participants: Arc::clone(&ports.participants),
            meetings: Arc::clone(&ports.meetings),
            summaries: Arc::clone(&ports.summaries),
        }
    }

    /// The transcript and template of `meeting`, or `None` when it has no transcript.
    async fn request(&self, meeting: MeetingId) -> Result<Option<SummaryInput>, StageError> {
        let segments = self
            .segments
            .list(&meeting, Page::ALL)
            .await
            .map_err(|error| StageError::retryable_from(&error))?;
        if segments.is_empty() {
            return Ok(None);
        }
        let speakers = self
            .participants
            .participants(&meeting)
            .await
            .map_err(|error| StageError::retryable_from(&error))?;
        let state = self
            .meetings
            .state(&meeting)
            .await
            .map_err(|error| StageError::retryable_from(&error))?;
        Ok(Some(SummaryInput {
            transcript: transcript_text(&segments, &names(speakers)),
            meeting_id: meeting,
            template: state.and_then(|state| state.template_id),
        }))
    }
}

#[async_trait]
impl PipelineStage for LegacySummaryStage {
    fn id(&self) -> StageId {
        StageId::new(ID)
    }

    fn order(&self) -> u16 {
        ORDER
    }

    async fn run(&self, ctx: &StageCtx, meeting: MeetingId) -> Result<StageOutcome, StageError> {
        if !self.summaries.is_automatic().await {
            return Ok(StageOutcome::Skipped(TURNED_OFF));
        }
        match self.request(meeting).await? {
            Some(request) => self.summaries.generate(request, ctx.cancel.clone()).await,
            None => Ok(StageOutcome::Skipped(NO_TRANSCRIPT)),
        }
    }
}

/// Each speaker's name: the person it is, else its label ("Speaker 2").
fn names(speakers: Vec<ParticipantRow>) -> HashMap<SpeakerId, String> {
    speakers
        .into_iter()
        .map(|row| {
            let name = row.person.map_or(row.speaker.label, |person| person.name);
            (row.speaker.id, name)
        })
        .collect()
}

/// The transcript as the legacy summariser expects it: `[MM:SS] text`
/// lines, with the speaker's name before the text when it is known.
fn transcript_text(segments: &[Segment], names: &HashMap<SpeakerId, String>) -> String {
    let lines: Vec<String> = segments
        .iter()
        .map(|segment| {
            let seconds = segment.start.get() / 1_000;
            let (minutes, seconds) = (seconds / SECONDS_PER_MINUTE, seconds % SECONDS_PER_MINUTE);
            let speaker = segment.speaker_id.and_then(|id| names.get(&id));
            match speaker {
                Some(name) => format!("[{minutes:02}:{seconds:02}] {name}: {}", segment.text),
                None => format!("[{minutes:02}:{seconds:02}] {}", segment.text),
            }
        })
        .collect();
    lines.join("\n")
}

#[cfg(any(test, feature = "test-support"))]
pub use recorded::RecordedSummaries;

#[cfg(any(test, feature = "test-support"))]
mod recorded {
    use std::sync::{Mutex, PoisonError};

    use async_trait::async_trait;
    use tokio_util::sync::CancellationToken;

    use super::{Summarizer, SummaryInput};
    use crate::{StageError, StageOutcome};

    /// A [`Summarizer`] for tests: records each request and succeeds.
    #[derive(Debug, Default)]
    pub struct RecordedSummaries {
        requests: Mutex<Vec<SummaryInput>>,
        turned_off: bool,
    }

    impl RecordedSummaries {
        /// A summariser whose user turned automatic summaries off.
        pub fn turned_off() -> Self {
            Self {
                turned_off: true,
                ..Self::default()
            }
        }

        /// Every request so far, oldest first.
        pub fn requests(&self) -> Vec<SummaryInput> {
            self.requests
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    #[async_trait]
    impl Summarizer for RecordedSummaries {
        async fn is_automatic(&self) -> bool {
            !self.turned_off
        }

        async fn generate(
            &self,
            request: SummaryInput,
            _cancel: CancellationToken,
        ) -> Result<StageOutcome, StageError> {
            self.requests
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(request);
            Ok(StageOutcome::Done)
        }
    }
}

#[cfg(test)]
mod tests {
    use deino_core::meetings::{FakeMeetingRepo, MeetingTitle, NewMeeting};
    use deino_core::segments::{FakeSegmentRepo, NewSegment, SegmentWriter, Speaker, SpeakerRepo};
    use deino_domain::{
        Channel, FakeClock, MeetingSource, MeetingStatus, Millis, PaletteColor, SegmentId,
    };
    use deino_testkit::at;

    use super::*;
    use crate::RecordedProgress;

    const AT: &str = "2026-10-08T09:00:00Z";

    struct Harness {
        stage: LegacySummaryStage,
        segments: Arc<FakeSegmentRepo>,
        meetings: Arc<FakeMeetingRepo>,
        summaries: Arc<RecordedSummaries>,
    }

    async fn harness() -> Harness {
        let segments = Arc::new(FakeSegmentRepo::default());
        let meetings = Arc::new(FakeMeetingRepo::default());
        let summaries = Arc::new(RecordedSummaries::default());
        meetings
            .create(&NewMeeting {
                id: MeetingId::new("meeting-a"),
                title: MeetingTitle::parse("Standup").unwrap(),
                status: MeetingStatus::Processing,
                source: MeetingSource::Live,
                created_at: at(AT),
                started_at: None,
                folder_path: None,
            })
            .await
            .unwrap();
        let ports = StagePorts {
            segments: segments.clone(),
            participants: segments.clone(),
            meetings: meetings.clone(),
            summaries: summaries.clone(),
            ..StagePorts::fakes()
        };
        let stage = LegacySummaryStage::new(&ports);
        Harness {
            stage,
            segments,
            meetings,
            summaries,
        }
    }

    fn segment(start_ms: u32, text: &str) -> NewSegment {
        let clock = FakeClock::new(at(AT));
        NewSegment {
            id: SegmentId::generate(&clock),
            meeting_id: MeetingId::new("meeting-a"),
            channel: Channel::Mixed,
            speaker_id: None,
            start: Millis(start_ms),
            end: Millis(start_ms + 500),
            text: text.into(),
            words: Vec::new(),
        }
    }

    fn ctx() -> StageCtx {
        StageCtx {
            attempt: 1,
            cancel: CancellationToken::new(),
            clock: Arc::new(FakeClock::new(at(AT))),
            events: Arc::new(RecordedProgress::default()),
        }
    }

    #[tokio::test]
    async fn a_meeting_without_transcript_is_skipped_without_calling_the_summariser() {
        let harness = harness().await;

        let outcome = harness.stage.run(&ctx(), MeetingId::new("meeting-a")).await;

        assert_eq!(outcome, Ok(StageOutcome::Skipped(NO_TRANSCRIPT)));
        assert!(harness.summaries.requests().is_empty());
    }

    #[tokio::test]
    async fn the_summariser_gets_timestamped_lines_and_the_meeting_template() {
        let harness = harness().await;
        let segments = [segment(1_250, "hello"), segment(61_000, "next item")];
        harness.segments.append(&segments).await.unwrap();
        let template = TemplateId::new("daily_standup");
        harness
            .meetings
            .set_template(&MeetingId::new("meeting-a"), Some(&template), at(AT))
            .await
            .unwrap();

        let outcome = harness.stage.run(&ctx(), MeetingId::new("meeting-a")).await;

        assert_eq!(outcome, Ok(StageOutcome::Done));
        assert_eq!(
            harness.summaries.requests(),
            [SummaryInput {
                meeting_id: MeetingId::new("meeting-a"),
                transcript: "[00:01] hello\n[01:01] next item".into(),
                template: Some(template),
            }]
        );
    }

    #[tokio::test]
    async fn turned_off_automatic_summaries_skip_the_stage() {
        let mut harness = harness().await;
        let summaries = Arc::new(RecordedSummaries::turned_off());
        harness.stage.summaries = summaries.clone();
        harness
            .segments
            .append(&[segment(0, "hello")])
            .await
            .unwrap();

        let outcome = harness.stage.run(&ctx(), MeetingId::new("meeting-a")).await;

        assert_eq!(outcome, Ok(StageOutcome::Skipped(TURNED_OFF)));
        assert!(summaries.requests().is_empty());
    }

    #[tokio::test]
    async fn lines_carry_the_speakers_names() {
        let harness = harness().await;
        let clock = FakeClock::new(at(AT));
        let speaker = Speaker {
            id: SpeakerId::generate(&clock),
            meeting_id: MeetingId::new("meeting-a"),
            label: "Speaker 2".into(),
            person_id: None,
            is_me: false,
            color: PaletteColor::cycle(1),
        };
        harness.segments.add_speaker(&speaker).await.unwrap();
        let said = NewSegment {
            speaker_id: Some(speaker.id),
            ..segment(2_000, "ship it")
        };
        harness
            .segments
            .append(&[said, segment(3_000, "ok")])
            .await
            .unwrap();

        harness
            .stage
            .run(&ctx(), MeetingId::new("meeting-a"))
            .await
            .unwrap();

        let transcript = &harness.summaries.requests()[0].transcript;
        assert_eq!(transcript, "[00:02] Speaker 2: ship it\n[00:03] ok");
    }

    #[test]
    fn the_stage_is_legacy_summary_at_order_fifty() {
        let ports = StagePorts {
            segments: Arc::new(FakeSegmentRepo::default()),
            meetings: Arc::new(FakeMeetingRepo::default()),
            summaries: Arc::new(RecordedSummaries::default()),
            ..StagePorts::fakes()
        };
        let stage = LegacySummaryStage::new(&ports);

        assert_eq!(
            (stage.id(), stage.order()),
            (StageId::new("legacy_summary"), 50)
        );
    }
}
