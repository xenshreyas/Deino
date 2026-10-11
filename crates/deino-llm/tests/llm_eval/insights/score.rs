//! Scoring insight answers: decisions against a fixture's ground truth, and
//! a discovery scorecard's fields against their types.

use std::ops::AddAssign;

use deino_core::insights::{FieldValue, InsightAnswer, InsightValue};
use deino_testkit::fixture::FixtureMeeting;

use crate::scoring::overlap;

/// The share of words an answer must share with a decision to be it.
const MATCH: f64 = 0.5;
/// The live bar: this share of the ground truth's decisions found.
const MIN_RECALL: f64 = 0.6;
/// The live bar: this share of answers attributed to a real moment.
const MIN_ATTRIBUTED: f64 = 0.9;

/// Counts over one or more meetings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Ground-truth decisions.
    pub decisions: usize,
    /// Of those, the ones an answer states.
    pub found: usize,
    /// Answers given.
    pub answers: usize,
    /// Of those, the ones with a speaker and a time.
    pub attributed: usize,
}

impl Tally {
    /// Found decisions over all decisions; 1 when there were none to find.
    pub fn recall(&self) -> f64 {
        ratio(self.found, self.decisions).max(f64::from(u8::from(self.decisions == 0)))
    }

    /// Attributed answers over all answers; 1 when there were none.
    pub fn attribution(&self) -> f64 {
        ratio(self.attributed, self.answers).max(f64::from(u8::from(self.answers == 0)))
    }

    /// A one-line summary.
    pub fn summary(&self) -> String {
        format!(
            "recall {:.2} ({}/{} decisions), attributed {:.2} ({}/{} answers)",
            self.recall(),
            self.found,
            self.decisions,
            self.attribution(),
            self.attributed,
            self.answers
        )
    }

    /// `Ok` when the counts reach the bars.
    pub fn passes(&self) -> Result<(), String> {
        if self.recall() >= MIN_RECALL && self.attribution() >= MIN_ATTRIBUTED {
            Ok(())
        } else {
            Err(self.summary())
        }
    }
}

impl AddAssign for Tally {
    fn add_assign(&mut self, other: Self) {
        self.decisions += other.decisions;
        self.found += other.found;
        self.answers += other.answers;
        self.attributed += other.attributed;
    }
}

/// The counts of `answers` against `meeting`'s decisions.
pub fn tally_decisions(meeting: &FixtureMeeting, answers: &[InsightAnswer]) -> Tally {
    let texts: Vec<String> = answers
        .iter()
        .map(|answer| answer.value.summary())
        .collect();
    let decisions = &meeting.ground_truth.decisions;
    Tally {
        decisions: decisions.len(),
        found: decisions
            .iter()
            .filter(|decision| texts.iter().any(|text| overlap(text, decision) >= MATCH))
            .count(),
        answers: answers.len(),
        attributed: answers
            .iter()
            .filter(|answer| answer.speaker.is_some() && answer.timestamp_ms.is_some())
            .count(),
    }
}

/// `answers` for `meeting` pass the bars on their own.
pub fn check_decisions(
    meeting: &FixtureMeeting,
    answers: &[InsightAnswer],
) -> Result<Tally, String> {
    let tally = tally_decisions(meeting, answers);
    tally.passes().map(|()| tally)
}

/// One scorecard whose fields are of their types and whose score is 1–5.
pub fn check_scorecard(answers: &[InsightAnswer]) -> Result<(), String> {
    let [answer] = answers else {
        return Err(format!("expected one scorecard, got {}", answers.len()));
    };
    if !matches!(answer.value, InsightValue::Fields { .. }) {
        return Err("the scorecard has no fields".into());
    }
    match answer.value.field("Score") {
        Some(FieldValue::Number(score)) if (1.0..=5.0).contains(score) => {}
        other => return Err(format!("the score is not 1–5: {other:?}")),
    }
    match answer.value.field("Pain") {
        Some(FieldValue::Text(pain)) if !pain.is_empty() => Ok(()),
        other => Err(format!("the pain is missing: {other:?}")),
    }
}

fn ratio(part: usize, whole: usize) -> f64 {
    #[allow(clippy::cast_precision_loss, reason = "counts are tiny")]
    let ratio = if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    };
    ratio
}
