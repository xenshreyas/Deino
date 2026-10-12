//! Which name wins each segment: the card's rule, pure.
//!
//! A name wins a segment when its active-speaker spans cover **more than
//! half** of the segment. Exactly half is not enough: two names sharing a
//! segment evenly is a crosstalk the app cannot settle, and the voice
//! matching that runs next decides it instead. A name's spans are summed, so
//! an indicator that flickers off for a poll still counts.

use deino_domain::{DisplayName, Millis, SegmentId, TagSpan};

/// A segment's place on the recording's timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timed {
    /// The segment.
    pub id: SegmentId,
    /// Its start.
    pub start: Millis,
    /// Its end; never before `start`.
    pub end: Millis,
}

impl Timed {
    /// How long the segment lasts.
    pub fn length(&self) -> u32 {
        self.end.0.saturating_sub(self.start.0)
    }
}

/// A segment won by a name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Win {
    /// The segment.
    pub segment: SegmentId,
    /// The name whose indicator was lit for most of it.
    pub name: DisplayName,
    /// How much of the segment the name's spans cover.
    pub covered: Millis,
}

/// Every segment a name wins (see the module docs), in `segments` order.
/// `spans` may come in any order; zero-length segments are never won.
pub fn align(spans: &[TagSpan], segments: &[Timed]) -> Vec<Win> {
    let mut sorted: Vec<&TagSpan> = spans.iter().collect();
    sorted.sort_by_key(|span| span.start);
    let longest = sorted.iter().map(|span| span.length()).max().unwrap_or(0);
    segments
        .iter()
        .filter_map(|segment| winner(&sorted, longest, segment))
        .collect()
}

/// The name covering more than half of `segment`, if any. `spans` are sorted
/// by start; none is longer than `longest`, so no span starting before
/// `segment.start - longest` can reach it.
fn winner(spans: &[&TagSpan], longest: u32, segment: &Timed) -> Option<Win> {
    let length = segment.length();
    if length == 0 {
        return None;
    }
    let earliest = Millis(segment.start.0.saturating_sub(longest));
    let first = spans.partition_point(|span| span.start < earliest);
    let mut covered: Vec<(&DisplayName, u32)> = Vec::new();
    for span in spans[first..]
        .iter()
        .take_while(|span| span.start < segment.end)
    {
        let overlap = span.overlap(segment.start, segment.end);
        match covered.iter_mut().find(|(name, _)| *name == &span.name) {
            Some((_, total)) => *total += overlap,
            None => covered.push((&span.name, overlap)),
        }
    }
    let (name, total) = covered.into_iter().max_by_key(|(_, total)| *total)?;
    (u64::from(total) * 2 > u64::from(length)).then(|| Win {
        segment: segment.id,
        name: name.clone(),
        covered: Millis(total),
    })
}

#[cfg(test)]
#[path = "align_tests.rs"]
mod tests;
