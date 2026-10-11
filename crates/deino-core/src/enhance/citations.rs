//! Citation validation: the model's transcript references (`s12`) back to
//! segments, keeping only those the meeting's transcript has.
//!
//! The model may answer `s12`, `S12`, `[s12]`, `s12 04:31` or invent `s999`;
//! every form that names a real line resolves, anything else is dropped
//! (PARITY_PLAN NOT-02: "validate every `segment_id`; drop invalid
//! citations"). A block keeps at most [`MAX_CITATIONS`], in the model's
//! order, without repeats.

use std::collections::BTreeMap;

use super::Evidence;
use super::transcript::TranscriptLine;

/// The most citations one block or detail keeps.
pub const MAX_CITATIONS: usize = 3;

/// The meeting's transcript lines by reference number.
#[derive(Clone, Debug, Default)]
pub struct SegmentIndex {
    by_reference: BTreeMap<u32, Evidence>,
}

impl SegmentIndex {
    /// The index of `lines`.
    pub fn new(lines: &[TranscriptLine]) -> Self {
        let by_reference = lines
            .iter()
            .map(|line| {
                let evidence = Evidence {
                    segment_id: line.segment_id,
                    start: line.start,
                };
                (line.reference, evidence)
            })
            .collect();
        Self { by_reference }
    }

    /// The segments `references` name, in order, without repeats or
    /// unknown references, at most [`MAX_CITATIONS`].
    pub fn resolve(&self, references: &[String]) -> Vec<Evidence> {
        let mut resolved: Vec<Evidence> = Vec::new();
        for reference in references {
            let Some(evidence) =
                parse_reference(reference).and_then(|number| self.by_reference.get(&number))
            else {
                continue;
            };
            if !resolved.contains(evidence) {
                resolved.push(*evidence);
            }
            if resolved.len() == MAX_CITATIONS {
                break;
            }
        }
        resolved
    }

    /// Whether `reference` names a line of this transcript.
    pub fn contains(&self, reference: &str) -> bool {
        parse_reference(reference).is_some_and(|number| self.by_reference.contains_key(&number))
    }
}

/// The number of the first `s<digits>` in `reference`: `s12`, `S12`,
/// `[s12]` and `s12 04:31` all name line 12. Insights (AI-06) resolve their
/// answers' references with it too.
pub fn parse_reference(reference: &str) -> Option<u32> {
    let lower = reference.to_ascii_lowercase();
    let start = lower.find('s')?;
    let digits: String = lower
        .get(start + 1..)?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use deino_domain::{Millis, SegmentId};

    use super::*;

    fn line(reference: u32) -> TranscriptLine {
        TranscriptLine {
            reference,
            segment_id: uuid::Uuid::from_u128(u128::from(reference))
                .to_string()
                .parse::<SegmentId>()
                .unwrap(),
            start: Millis(reference * 1_000),
            end: Millis(reference * 1_000 + 500),
            speaker: "Maya".into(),
            text: "text".into(),
        }
    }

    fn references(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn every_form_of_a_real_reference_resolves_in_order() {
        let index = SegmentIndex::new(&[line(3), line(12)]);

        let evidence = index.resolve(&references(&["[s12]", "S3 00:03"]));

        assert_eq!(
            evidence.iter().map(|e| e.start).collect::<Vec<_>>(),
            [Millis(12_000), Millis(3_000)]
        );
    }

    #[test]
    fn invented_malformed_and_repeated_references_are_dropped() {
        let index = SegmentIndex::new(&[line(1), line(2)]);

        let evidence = index.resolve(&references(&["s999", "segment", "", "s1", "s1"]));

        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].start, Millis(1_000));
        assert!(!index.contains("s999"));
    }

    #[test]
    fn a_block_keeps_at_most_three_citations() {
        let lines: Vec<TranscriptLine> = (1..=5).map(line).collect();
        let index = SegmentIndex::new(&lines);

        let evidence = index.resolve(&references(&["s1", "s2", "s3", "s4", "s5"]));

        assert_eq!(evidence.len(), MAX_CITATIONS);
    }
}
