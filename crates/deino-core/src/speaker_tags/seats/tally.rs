//! The ballot's sums, counted once, so the rules stay linear in the
//! meeting's length.

use std::cmp::Reverse;
use std::collections::HashMap;

use deino_domain::{DisplayName, SegmentId, SpeakerId};

use super::{Ballot, Seated};
use crate::segments::Speaker;

/// Shares are compared in thousandths.
const PER_MILLE: u64 = 1_000;

/// Speech per speaker and per name, from one pass over the ballot.
pub(super) struct Tally<'a> {
    /// Each segment by ID.
    segments: HashMap<SegmentId, &'a Seated>,
    /// Each speaker's speech on the track.
    speech: HashMap<SpeakerId, u64>,
    /// The speech of each speaker that each name won.
    won: HashMap<(SpeakerId, &'a DisplayName), u64>,
    /// Each name's won speech, in first-win order.
    totals: Vec<(&'a DisplayName, u64)>,
}

impl<'a> Tally<'a> {
    /// Counts `ballot`.
    pub(super) fn count(ballot: &Ballot<'a>) -> Self {
        let segments: HashMap<SegmentId, &Seated> = ballot
            .segments
            .iter()
            .map(|segment| (segment.id, segment))
            .collect();
        let mut speech: HashMap<SpeakerId, u64> = HashMap::new();
        for segment in ballot.segments {
            if let Some(speaker) = segment.speaker {
                *speech.entry(speaker).or_default() += u64::from(segment.length);
            }
        }
        let mut tally = Self {
            segments,
            speech,
            won: HashMap::new(),
            totals: Vec::new(),
        };
        for win in ballot.wins {
            tally.add_win(&win.name, win.segment);
        }
        tally
    }

    fn add_win(&mut self, name: &'a DisplayName, segment: SegmentId) {
        let Some(segment) = self.segments.get(&segment).copied() else {
            return;
        };
        let length = u64::from(segment.length);
        if let Some(speaker) = segment.speaker {
            *self.won.entry((speaker, name)).or_default() += length;
        }
        match self.totals.iter_mut().find(|(known, _)| *known == name) {
            Some((_, total)) => *total += length,
            None => self.totals.push((name, length)),
        }
    }

    /// The segment with this ID.
    pub(super) fn segment(&self, id: SegmentId) -> Option<&'a Seated> {
        self.segments.get(&id).copied()
    }

    /// The distinct winning names, most won speech first, then by name.
    pub(super) fn ranked_names(&self) -> Vec<DisplayName> {
        let mut totals = self.totals.clone();
        totals.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        totals.into_iter().map(|(name, _)| name.clone()).collect()
    }

    /// Rule 3: the candidate `name` wins the largest share of, if that share
    /// is more than half of the speaker's speech.
    pub(super) fn majority_speaker(
        &self,
        candidates: &[&Speaker],
        name: &DisplayName,
    ) -> Option<SpeakerId> {
        candidates
            .iter()
            .filter_map(|speaker| {
                let speech = self.speech.get(&speaker.id).copied().unwrap_or(0);
                let won = self.won.get(&(speaker.id, name)).copied().unwrap_or(0);
                (won * 2 > speech).then_some((speaker.id, won * PER_MILLE / speech.max(1)))
            })
            .max_by_key(|&(speaker, share)| (share, Reverse(speaker)))
            .map(|(speaker, _)| speaker)
    }
}
