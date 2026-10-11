//! What each lane is hearing and transcribing, for the watchers of a
//! recording: recording health ("speech but no transcript") and auto-stop
//! ("no speech for 15 minutes", CAP-07).
//!
//! The transcript router is the only writer (it sees every voice change,
//! text and watermark of both lanes, in order); readers on other threads
//! take a [`LaneReading`] at any time. Times are on the recording's
//! timeline, so a pause or a slow lane never reads as silence.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use deino_stt::Millis;

use crate::lane::{Lane, PerLane};
use crate::vad::VoiceChange;

/// Marks "no open span" in [`LaneActivity::open_since`].
const NONE: u64 = u64::MAX;

/// One lane's activity, as the router last saw it.
#[derive(Debug)]
struct LaneActivity {
    speaking: AtomicBool,
    /// Milliseconds of closed voice spans, in total.
    voiced: AtomicU64,
    /// Where the open voice span started, or [`NONE`].
    open_since: AtomicU64,
    /// The lane's transcript watermark.
    through: AtomicU64,
    /// Milliseconds of closed voice spans since the lane's latest text.
    unanswered: AtomicU64,
    /// The watermark when the lane's latest text arrived.
    answered_at: AtomicU64,
}

impl Default for LaneActivity {
    fn default() -> Self {
        Self {
            speaking: AtomicBool::new(false),
            voiced: AtomicU64::new(0),
            open_since: AtomicU64::new(NONE),
            through: AtomicU64::new(0),
            unanswered: AtomicU64::new(0),
            answered_at: AtomicU64::new(0),
        }
    }
}

/// A lane's activity at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LaneReading {
    /// Someone is speaking on the lane now.
    pub speaking: bool,
    /// Speech heard on the lane so far (it only grows).
    pub voiced: Millis,
    /// Speech heard since the lane last produced any text.
    pub unanswered: Millis,
}

/// Both lanes' activity, shared between the router and the watchers.
#[derive(Debug, Default)]
pub struct LiveActivity {
    lanes: PerLane<LaneActivity>,
}

impl LiveActivity {
    /// A lane's voice detector changed state.
    pub(crate) fn voice(&self, lane: Lane, change: VoiceChange) {
        let activity = self.lanes.get(lane);
        match change {
            VoiceChange::Started(at) => {
                activity.open_since.store(at.0, Ordering::Release);
                activity.speaking.store(true, Ordering::Release);
            }
            VoiceChange::Ended(span) => {
                let answered = activity.answered_at.load(Ordering::Acquire);
                let length = span.end.0.saturating_sub(span.start.0);
                let fresh = span.end.0.saturating_sub(span.start.0.max(answered));
                activity.voiced.fetch_add(length, Ordering::AcqRel);
                activity.unanswered.fetch_add(fresh, Ordering::AcqRel);
                activity.open_since.store(NONE, Ordering::Release);
                activity.speaking.store(false, Ordering::Release);
            }
        }
    }

    /// The lane produced text (live or final).
    pub(crate) fn text(&self, lane: Lane) {
        let activity = self.lanes.get(lane);
        let through = activity.through.load(Ordering::Acquire);
        activity.answered_at.store(through, Ordering::Release);
        activity.unanswered.store(0, Ordering::Release);
    }

    /// The lane's transcript watermark moved.
    pub(crate) fn progress(&self, lane: Lane, through: Millis) {
        self.lanes
            .get(lane)
            .through
            .fetch_max(through.0, Ordering::AcqRel);
    }

    /// `lane`'s activity now; an open span counts up to the watermark.
    pub fn read(&self, lane: Lane) -> LaneReading {
        let activity = self.lanes.get(lane);
        let through = activity.through.load(Ordering::Acquire);
        let open_since = activity.open_since.load(Ordering::Acquire);
        let answered = activity.answered_at.load(Ordering::Acquire);
        let (open, open_unanswered) = if open_since == NONE {
            (0, 0)
        } else {
            let unanswered_from = open_since.max(answered);
            (
                through.saturating_sub(open_since),
                through.saturating_sub(unanswered_from),
            )
        };
        LaneReading {
            speaking: activity.speaking.load(Ordering::Acquire),
            voiced: Millis(activity.voiced.load(Ordering::Acquire) + open),
            unanswered: Millis(activity.unanswered.load(Ordering::Acquire) + open_unanswered),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vad::VoiceSpan;

    fn span(start: u64, end: u64) -> VoiceChange {
        VoiceChange::Ended(VoiceSpan {
            start: Millis(start),
            end: Millis(end),
        })
    }

    #[test]
    fn closed_spans_add_up_and_an_open_one_counts_to_the_watermark() {
        let activity = LiveActivity::default();
        activity.voice(Lane::Me, VoiceChange::Started(Millis(1_000)));
        activity.voice(Lane::Me, span(1_000, 3_000));
        activity.voice(Lane::Me, VoiceChange::Started(Millis(5_000)));
        activity.progress(Lane::Me, Millis(6_500));

        let reading = activity.read(Lane::Me);

        assert!(reading.speaking);
        assert_eq!(reading.voiced, Millis(3_500));
        assert_eq!(reading.unanswered, Millis(3_500));
        assert_eq!(activity.read(Lane::Them), LaneReading::default());
    }

    #[test]
    fn text_answers_the_speech_before_it() {
        let activity = LiveActivity::default();
        activity.voice(Lane::Me, VoiceChange::Started(Millis(0)));
        activity.progress(Lane::Me, Millis(4_000));
        activity.text(Lane::Me);
        activity.progress(Lane::Me, Millis(10_000));
        activity.voice(Lane::Me, span(0, 10_000));

        let reading = activity.read(Lane::Me);

        assert_eq!(reading.voiced, Millis(10_000));
        assert_eq!(reading.unanswered, Millis(6_000), "only after the text");
        assert!(!reading.speaking);
    }
}
