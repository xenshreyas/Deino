//! Attribution: an answer's transcript reference (`s12`, the format NOT-02's
//! notes cite with) resolved in Rust to who said it and when.
//!
//! The model only ever names a line; the speaker, the time and the segment
//! come from the transcript, so an answer can never be attributed to a
//! moment that did not happen. The first reference that names a real line
//! wins; an answer whose references name none stays unattributed.

use std::collections::BTreeMap;

use deino_domain::{Millis, SegmentId};

use super::InsightAnswer;
use super::validate::Validated;
use crate::enhance::{TranscriptLine, parse_reference};

/// One line of the transcript, as an answer is attributed to it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Moment {
    segment_id: SegmentId,
    start: Millis,
    speaker: String,
}

/// The meeting's transcript lines by reference number.
#[derive(Clone, Debug, Default)]
pub struct Attribution {
    by_reference: BTreeMap<u32, Moment>,
}

impl Attribution {
    /// The index of `lines` (their `reference` numbers, from 1).
    pub fn new(lines: &[TranscriptLine]) -> Self {
        let by_reference = lines
            .iter()
            .map(|line| {
                let moment = Moment {
                    segment_id: line.segment_id,
                    start: line.start,
                    speaker: line.speaker.clone(),
                };
                (line.reference, moment)
            })
            .collect();
        Self { by_reference }
    }

    /// `answer` with the speaker, time and segment of the first line its
    /// references name.
    pub fn attribute(&self, answer: Validated) -> InsightAnswer {
        let moment = answer
            .references
            .iter()
            .filter_map(|reference| parse_reference(reference))
            .find_map(|number| self.by_reference.get(&number));
        InsightAnswer {
            value: answer.value,
            speaker: moment.map(|moment| moment.speaker.clone()),
            timestamp_ms: moment.map(|moment| moment.start),
            segment_id: moment.map(|moment| moment.segment_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::insights::InsightValue;

    fn line(reference: u32, speaker: &str) -> TranscriptLine {
        TranscriptLine {
            reference,
            segment_id: uuid::Uuid::from_u128(u128::from(reference))
                .to_string()
                .parse()
                .unwrap(),
            start: Millis(reference * 10_000),
            end: Millis(reference * 10_000 + 4_000),
            speaker: speaker.into(),
            text: "text".into(),
        }
    }

    fn validated(references: &[&str]) -> Validated {
        Validated {
            value: InsightValue::Text {
                text: "Okta SSO".into(),
            },
            references: references.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn the_first_reference_that_names_a_line_gives_the_speaker_and_time() {
        let attribution = Attribution::new(&[line(3, "Maya Chen"), line(12, "Owen Hart")]);

        let answer = attribution.attribute(validated(&["s999", "[S12 02:00]", "s3"]));

        assert_eq!(answer.speaker.as_deref(), Some("Owen Hart"));
        assert_eq!(answer.timestamp_ms, Some(Millis(120_000)));
        assert_eq!(answer.segment_id, Some(line(12, "Owen Hart").segment_id));
    }

    #[test]
    fn an_answer_whose_references_name_no_line_stays_unattributed() {
        let attribution = Attribution::new(&[line(1, "Maya Chen")]);

        for references in [vec![], vec!["s7"], vec!["the intro"]] {
            let answer = attribution.attribute(validated(&references));
            assert_eq!(
                (answer.speaker, answer.timestamp_ms, answer.segment_id),
                (None, None, None),
                "{references:?}"
            );
        }
    }
}
