//! Each lane's open segment, kept as a draft row until it closes.

use deino_core::segments::{NewSegment, WordTiming};
use deino_domain::{MeetingId, Millis, SegmentId, SpeakerId};
use deino_stt::{FinalSegment, LiveText, TimedWord};

use crate::lane::{Lane, PerLane};

/// A lane's open segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Draft {
    /// The row it is (or will be) saved as; the final keeps it.
    pub id: SegmentId,
    /// When it starts.
    pub start: deino_stt::Millis,
    /// Its committed text so far.
    pub text: String,
    /// Changed since it was last saved.
    pub dirty: bool,
    /// Saved at least once (so discarding it must delete the row).
    pub saved: bool,
}

/// Both lanes' drafts.
#[derive(Debug, Default)]
pub(super) struct Drafts {
    lanes: PerLane<Option<Draft>>,
}

impl Drafts {
    /// Records `lane`'s open segment; returns the draft when it has text.
    /// A segment starting somewhere new replaces the old draft (whose row,
    /// if saved, keeps its last text).
    pub fn update(
        &mut self,
        lane: Lane,
        text: &LiveText,
        id: impl FnOnce() -> SegmentId,
    ) -> Option<&Draft> {
        let committed = text.committed.trim();
        if committed.is_empty() {
            return None;
        }
        let slot = self.lanes.get_mut(lane);
        if slot.as_ref().is_none_or(|draft| draft.start != text.start) {
            *slot = Some(Draft {
                id: id(),
                start: text.start,
                text: String::new(),
                dirty: false,
                saved: false,
            });
        }
        let draft = slot.as_mut()?;
        if draft.text != committed {
            committed.clone_into(&mut draft.text);
            draft.dirty = true;
        }
        Some(draft)
    }

    /// Takes `lane`'s draft for a segment that closed at `start`.
    pub fn close(&mut self, lane: Lane, start: deino_stt::Millis) -> Option<Draft> {
        let slot = self.lanes.get_mut(lane);
        slot.take_if(|draft| draft.start == start)
    }

    /// `lane`'s draft, if it has unsaved changes.
    pub fn unsaved(&mut self, lane: Lane) -> Option<&mut Draft> {
        self.lanes
            .get_mut(lane)
            .as_mut()
            .filter(|draft| draft.dirty)
    }
}

/// Where a lane's segments are stored.
#[derive(Clone, Debug)]
pub(super) struct Owner {
    pub meeting: MeetingId,
    pub speakers: PerLane<Option<SpeakerId>>,
}

impl Owner {
    /// The row for a draft.
    pub fn draft_row(&self, lane: Lane, draft: &Draft) -> NewSegment {
        NewSegment {
            id: draft.id,
            meeting_id: self.meeting.clone(),
            channel: lane.channel(),
            speaker_id: *self.speakers.get(lane),
            start: millis(draft.start),
            end: millis(draft.start),
            text: draft.text.clone(),
            words: Vec::new(),
        }
    }

    /// The row for a closed segment.
    pub fn final_row(&self, lane: Lane, id: SegmentId, segment: &FinalSegment) -> NewSegment {
        NewSegment {
            id,
            meeting_id: self.meeting.clone(),
            channel: lane.channel(),
            speaker_id: *self.speakers.get(lane),
            start: millis(segment.start),
            end: millis(segment.end.max(segment.start)),
            text: segment.text.trim().to_owned(),
            words: segment.words.iter().map(word).collect(),
        }
    }
}

/// An engine time as a stored time (saturating past 49 days).
pub(super) fn millis(time: deino_stt::Millis) -> Millis {
    Millis(u32::try_from(time.0).unwrap_or(u32::MAX))
}

fn word(word: &TimedWord) -> WordTiming {
    WordTiming {
        text: word.text.clone(),
        start: millis(word.start),
        end: millis(word.end.max(word.start)),
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;

    fn live(start: u64, text: &str) -> LiveText {
        LiveText {
            start: deino_stt::Millis(start),
            committed: text.into(),
            tentative: String::new(),
        }
    }

    fn new_id() -> SegmentId {
        SegmentId::generate(&FakeClock::new(at("2026-10-09T10:00:00Z")))
    }

    #[test]
    fn a_draft_keeps_its_id_while_its_segment_grows() {
        let mut drafts = Drafts::default();
        let first = drafts
            .update(Lane::Me, &live(1_000, "so"), new_id)
            .unwrap()
            .id;
        let grown = drafts
            .update(Lane::Me, &live(1_000, "so the plan"), new_id)
            .unwrap()
            .clone();
        assert_eq!(grown.id, first);
        assert_eq!(grown.text, "so the plan");
        assert!(grown.dirty);
        assert_eq!(
            drafts
                .close(Lane::Me, deino_stt::Millis(1_000))
                .map(|d| d.id),
            Some(first)
        );
        assert!(drafts.unsaved(Lane::Me).is_none());
    }

    #[test]
    fn empty_text_is_no_draft_and_a_new_start_is_a_new_draft() {
        let mut drafts = Drafts::default();
        assert!(drafts.update(Lane::Them, &live(0, "  "), new_id).is_none());
        let first = drafts
            .update(Lane::Them, &live(0, "hello"), new_id)
            .unwrap()
            .id;
        let second = drafts
            .update(Lane::Them, &live(5_000, "next"), new_id)
            .unwrap()
            .id;
        assert_ne!(first, second);
        assert!(drafts.close(Lane::Them, deino_stt::Millis(0)).is_none());
    }

    #[test]
    fn engine_times_saturate_into_stored_times() {
        assert_eq!(millis(deino_stt::Millis(1_500)), Millis(1_500));
        assert_eq!(millis(deino_stt::Millis(u64::MAX)), Millis(u32::MAX));
    }
}
