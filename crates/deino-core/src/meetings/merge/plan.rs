//! Planning a merge: which meeting absorbs the others, and where each lands
//! on the merged timeline. Pure, so every rule is tested without a database.
//!
//! Meetings are concatenated in the order they took place: the earliest
//! keeps its ID and starts at 0, and each later one starts where the
//! previous one ended, so transcript times stay unique and each part's
//! recording plays from its offset.

use deino_domain::{MeetingId, MeetingStatus, Millis};

use super::{MergeCandidate, MergePart};
use crate::meetings::MeetingError;

/// The timeline a merge produces, before notes and the clock are added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timeline {
    /// The meeting that absorbs the others.
    pub target: MeetingId,
    /// The absorbed meetings, in time order.
    pub parts: Vec<MergePart>,
    /// The whole timeline's length.
    pub length: Millis,
}

/// Orders `candidates` and lays them end to end. Fails unless there are at
/// least two, all outside the trash and finished (ready, or failed with
/// whatever they kept).
pub fn timeline(mut candidates: Vec<MergeCandidate>) -> Result<Timeline, MeetingError> {
    if candidates.len() < 2 {
        return Err(MeetingError::MergeNeedsTwo);
    }
    if let Some(busy) = candidates.iter().find(|candidate| !mergeable(candidate)) {
        return Err(MeetingError::NotMergeable(busy.id.clone()));
    }
    candidates.sort_by(|a, b| (a.held_at, &a.id).cmp(&(b.held_at, &b.id)));
    let mut rest = candidates.into_iter();
    let Some(first) = rest.next() else {
        return Err(MeetingError::MergeNeedsTwo);
    };
    let (mut offset, mut position) = (first.length.get(), first.parts + 1);
    let mut parts = Vec::new();
    for candidate in rest {
        let part = MergePart {
            source: candidate.id,
            position,
            title: candidate.title,
            held_at: candidate.held_at,
            offset: Millis(offset),
            length: candidate.length,
            audio: candidate.audio,
        };
        offset = offset.saturating_add(part.length.get());
        position += 1 + candidate.parts;
        parts.push(part);
    }
    Ok(Timeline {
        target: first.id,
        parts,
        length: Millis(offset),
    })
}

/// Finished and outside the trash: recording or processing meetings are
/// still being written, and a scheduled one has nothing to merge yet.
fn mergeable(candidate: &MergeCandidate) -> bool {
    !candidate.trashed
        && matches!(
            candidate.status,
            MeetingStatus::Ready | MeetingStatus::Failed
        )
}

#[cfg(test)]
mod tests {
    use deino_testkit::at;

    use super::*;
    use crate::meetings::MeetingAudio;

    fn candidate(id: &str, held_at: &str, length: u32) -> MergeCandidate {
        MergeCandidate {
            id: MeetingId::new(id),
            title: id.to_uppercase(),
            held_at: at(held_at),
            status: MeetingStatus::Ready,
            trashed: false,
            length: Millis(length),
            parts: 0,
            audio: MeetingAudio::default(),
        }
    }

    #[test]
    fn the_earliest_absorbs_the_rest_laid_end_to_end_in_time_order() {
        let timeline = timeline(vec![
            candidate("c", "2026-10-03T09:00:00Z", 30_000),
            candidate("a", "2026-10-01T09:00:00Z", 60_000),
            candidate("b", "2026-10-02T09:00:00Z", 45_000),
        ])
        .unwrap();

        assert_eq!(timeline.target.as_str(), "a");
        let placed: Vec<(&str, u32, u32)> = timeline
            .parts
            .iter()
            .map(|part| (part.source.as_str(), part.position, part.offset.get()))
            .collect();
        assert_eq!(placed, [("b", 1, 60_000), ("c", 2, 105_000)]);
        assert_eq!(timeline.length, Millis(135_000));
    }

    #[test]
    fn earlier_merges_keep_their_part_numbers_free() {
        let mut target = candidate("a", "2026-10-01T09:00:00Z", 10);
        target.parts = 2;
        let mut merged = candidate("b", "2026-10-02T09:00:00Z", 10);
        merged.parts = 1;

        let timeline = timeline(vec![
            target,
            merged,
            candidate("c", "2026-10-03T09:00:00Z", 10),
        ])
        .unwrap();

        let positions: Vec<u32> = timeline.parts.iter().map(|part| part.position).collect();
        assert_eq!(positions, [3, 5], "b's own part takes 4");
    }

    #[test]
    fn one_meeting_or_an_unfinished_or_trashed_one_is_refused() {
        let mut recording = candidate("b", "2026-10-02T09:00:00Z", 10);
        recording.status = MeetingStatus::Processing;
        let mut trashed = candidate("c", "2026-10-03T09:00:00Z", 10);
        trashed.trashed = true;
        let first = || candidate("a", "2026-10-01T09:00:00Z", 10);

        assert!(matches!(
            timeline(vec![first()]),
            Err(MeetingError::MergeNeedsTwo)
        ));
        assert!(matches!(
            timeline(vec![first(), recording]),
            Err(MeetingError::NotMergeable(id)) if id.as_str() == "b"
        ));
        assert!(matches!(
            timeline(vec![first(), trashed]),
            Err(MeetingError::NotMergeable(_))
        ));
    }
}
