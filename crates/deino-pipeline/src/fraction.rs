//! [`StageFraction`]: how far a running stage has got, reported through
//! the stage's [`ProgressSink`] as a running [`PipelineProgress`] with a
//! fraction, at most once per whole percent.
//!
//! A stage reports as often as its work allows (after every utterance, for
//! `final_transcription`); the UI only needs a new number when the
//! percent it shows changes, so everything in between is dropped here.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use deino_domain::{JobStatus, MeetingId, StageId};

use crate::events::whole_percent;
use crate::{PipelineProgress, ProgressSink};

/// No percent reported yet.
const NONE: u8 = u8::MAX;

/// Reports one stage's progress for one meeting.
pub struct StageFraction {
    sink: Arc<dyn ProgressSink>,
    meeting: MeetingId,
    stage: StageId,
    last: AtomicU8,
}

impl StageFraction {
    /// Progress of `stage` for `meeting`, reported to `sink`.
    pub fn new(sink: Arc<dyn ProgressSink>, meeting: MeetingId, stage: StageId) -> Self {
        Self {
            sink,
            meeting,
            stage,
            last: AtomicU8::new(NONE),
        }
    }

    /// The stage is `fraction` (0.0 to 1.0) done. Reported only when the
    /// whole percent differs from the last one reported; never blocks.
    pub fn report(&self, fraction: f32) {
        let percent = whole_percent(fraction);
        if self.last.swap(percent, Ordering::Relaxed) == percent {
            return;
        }
        self.sink.progress(&PipelineProgress {
            meeting_id: self.meeting.clone(),
            stage: self.stage.clone(),
            status: JobStatus::Running,
            fraction: Some(fraction.clamp(0.0, 1.0)),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecordedProgress;

    #[test]
    fn only_a_new_whole_percent_is_reported() {
        let events = Arc::new(RecordedProgress::default());
        let stage = StageId::new("final_transcription");
        let fraction = StageFraction::new(events.clone(), MeetingId::new("m"), stage.clone());

        for reported in [0.0, 0.001, 0.004, 0.4, 0.401, 0.409, 0.415, 0.999, 1.0, 1.0] {
            fraction.report(reported);
        }

        let events = events.events();
        let percents: Vec<Option<u8>> = events.iter().map(PipelineProgress::percent).collect();
        assert_eq!(
            percents,
            [Some(0), Some(40), Some(41), Some(99), Some(100)].to_vec()
        );
        assert!(
            events
                .iter()
                .all(|event| event.status == JobStatus::Running)
        );
        assert!(events.iter().all(|event| event.stage == stage));
    }

    #[test]
    fn a_plain_status_change_has_no_percent() {
        let progress = PipelineProgress::status(
            MeetingId::new("m"),
            StageId::new("index"),
            JobStatus::Succeeded,
        );

        assert_eq!(progress.percent(), None);
    }
}
