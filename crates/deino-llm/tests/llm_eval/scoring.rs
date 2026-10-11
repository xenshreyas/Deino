//! Scoring extracted action items against a fixture's `groundTruth`:
//! one-to-one matching, then precision and recall, plus how often a matched
//! item has the right assignee and the right due day.
//!
//! A prediction matches a ground-truth item when its evidence quote or its
//! title shares most of the other's meaningful words (the smaller side's
//! words, so a longer paraphrase still counts). Pairs are taken best first,
//! each side at most once, so two items cannot claim one commitment.

use std::collections::BTreeSet;
use std::ops::AddAssign;

use deino_llm::extraction::ExtractedActionItem;
use deino_testkit::fixture::{ExpectedActionItem, FixtureMeeting, USER_EMAIL};

/// The share of words two texts must share to be the same commitment.
const MATCH: f64 = 0.5;

/// Words too common to tell two commitments apart.
const STOP_WORDS: [&str; 24] = [
    "a", "an", "and", "at", "by", "for", "from", "i", "i'll", "in", "it", "me", "my", "of", "on",
    "so", "that", "the", "this", "to", "we", "we'll", "with", "you",
];

/// Counts over one or more meetings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Score {
    /// Predictions that matched a ground-truth item.
    pub matched: usize,
    /// Predictions that matched nothing.
    pub extra: usize,
    /// Ground-truth items nothing matched.
    pub missed: usize,
    /// Matched items with the right assignee.
    pub right_assignee: usize,
    /// Matched items with the right due day (the model's own `due`).
    pub right_due: usize,
}

impl Score {
    /// Matched predictions over all predictions (0 when there are none).
    pub fn precision(&self) -> f64 {
        ratio(self.matched, self.matched + self.extra)
    }

    /// Matched ground truth over all ground truth.
    pub fn recall(&self) -> f64 {
        ratio(self.matched, self.matched + self.missed)
    }

    /// A one-line summary.
    pub fn summary(&self) -> String {
        format!(
            "precision {:.2}, recall {:.2} ({} matched, {} extra, {} missed); assignee right {:.2}, due right {:.2}",
            self.precision(),
            self.recall(),
            self.matched,
            self.extra,
            self.missed,
            ratio(self.right_assignee, self.matched),
            ratio(self.right_due, self.matched),
        )
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, other: Self) {
        self.matched += other.matched;
        self.extra += other.extra;
        self.missed += other.missed;
        self.right_assignee += other.right_assignee;
        self.right_due += other.right_due;
    }
}

/// Scores `predicted` against `meeting`'s ground truth.
pub fn score(meeting: &FixtureMeeting, predicted: &[ExtractedActionItem]) -> Score {
    let truth = &meeting.ground_truth.action_items;
    let pairs = pairs(predicted, truth);
    let mut score = Score {
        matched: pairs.len(),
        extra: predicted.len() - pairs.len(),
        missed: truth.len() - pairs.len(),
        ..Score::default()
    };
    for &(p, t) in &pairs {
        let (Some(guess), Some(expected)) = (predicted.get(p), truth.get(t)) else {
            continue;
        };
        let owner = expected
            .assignee
            .as_ref()
            .map(|email| email.as_str().to_owned());
        score.right_assignee += usize::from(assignee_email(meeting, guess) == owner);
        let due = guess.due.as_deref().and_then(|due| due.parse().ok());
        score.right_due += usize::from(due == expected.due);
    }
    score
}

/// One-to-one (prediction, truth) index pairs, best similarity first.
fn pairs(predicted: &[ExtractedActionItem], truth: &[ExpectedActionItem]) -> Vec<(usize, usize)> {
    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
    for (p, guess) in predicted.iter().enumerate() {
        for (t, expected) in truth.iter().enumerate() {
            let similarity = overlap(&guess.evidence_quote, &expected.evidence)
                .max(overlap(&guess.title, &expected.title));
            if similarity >= MATCH {
                candidates.push((similarity, p, t));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (mut used_p, mut used_t) = (BTreeSet::new(), BTreeSet::new());
    let mut chosen = Vec::new();
    for (_, p, t) in candidates {
        if !used_p.contains(&p) && !used_t.contains(&t) {
            used_p.insert(p);
            used_t.insert(t);
            chosen.push((p, t));
        }
    }
    chosen
}

/// The share of the smaller text's meaningful words found in the other.
pub fn overlap(a: &str, b: &str) -> f64 {
    let (a, b) = (meaningful(a), meaningful(b));
    let shared = a.intersection(&b).count();
    ratio(shared, a.len().min(b.len()))
}

fn meaningful(text: &str) -> BTreeSet<String> {
    text.to_lowercase()
        .split(|character: char| !character.is_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty() && !STOP_WORDS.contains(word))
        .map(str::to_owned)
        .collect()
}

/// The address of the attendee a predicted assignee names ("me", a full or
/// first name).
fn assignee_email(meeting: &FixtureMeeting, guess: &ExtractedActionItem) -> Option<String> {
    let said = guess.assignee.as_deref()?.trim().to_lowercase();
    if said == "me" {
        return Some(USER_EMAIL.to_owned());
    }
    meeting
        .attendees
        .iter()
        .find(|attendee| {
            let name = attendee.name.to_lowercase();
            name == said || name.split(' ').next() == Some(said.as_str())
        })
        .map(|attendee| attendee.email.as_str().to_owned())
}

fn ratio(part: usize, whole: usize) -> f64 {
    #[allow(clippy::cast_precision_loss, reason = "item counts are tiny")]
    let ratio = if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    };
    ratio
}
