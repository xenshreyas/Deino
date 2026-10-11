//! The naming rules of `identify_speakers`, pure and generic over the
//! speaker and person keys (tests use `u8` and `char`).
//!
//! From `docs/references/spike-diarization.md` (WeSpeaker ResNet34-LM):
//!
//! 1. **By voice.** A speaker is a person when the person's best exemplar
//!    scores at least `autoAssign` (`autoAssignNonAttendee` for someone not
//!    invited), beats every other candidate by `margin`, **and** the person
//!    has no closer speaker (mutual best, [`pick_unique`]). Both safeguards
//!    are needed: a phantom cluster contaminated with another voice matched
//!    the wrong person at 0.81 in the spike, and only the two together
//!    rejected it.
//! 2. **By elimination.** When a track holds exactly as many speakers as
//!    attendees expected on it and all but one of each are paired, the last
//!    speaker is the last attendee ([`complete_by_elimination`]), voiceprint
//!    or not.
//! 3. **Suggestions.** Every speaker still unnamed whose best remaining
//!    candidate scores at least `suggest` gets "Is this …?"; each person is
//!    suggested once, to the speaker that resembles them most.

use deino_domain::Millis;

use crate::settings::IdentifyThresholds;
use crate::voiceprints::matching::{MatchRule, complete_by_elimination, pick_unique};

/// A diarized voice with less speech than this is too thin to identify (a
/// phantom cluster's worth): it is neither named by voice nor suggested.
pub const MIN_IDENTIFIABLE_SPEECH: Millis = Millis(3_000);

/// What the rules look at.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ballot<S, P> {
    /// One `(speaker, person, score)` per unnamed voice and candidate: the
    /// person's best exemplar's cosine similarity.
    pub scores: Vec<(S, P, f32)>,
    /// The meeting's attendees who did not decline.
    pub attendees: Vec<P>,
    /// The diarized track's speakers and the attendees expected on it.
    pub roster: Roster<S, P>,
}

/// One diarized track, for naming by elimination.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roster<S, P> {
    /// Every speaker on the track.
    pub speakers: Vec<S>,
    /// The attendees expected on it (the user excluded on a call).
    pub expected: Vec<P>,
    /// Speakers already named, by the user or an earlier run.
    pub named: Vec<(S, P)>,
}

/// How a speaker was named.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NamedBy {
    /// Their voice matched, with this score.
    Voice(f32),
    /// They were the last unpaired attendee.
    Elimination,
}

/// One speaker named automatically.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Naming<S, P> {
    /// The speaker.
    pub speaker: S,
    /// Who it is.
    pub person: P,
    /// Why.
    pub by: NamedBy,
}

/// What the rules decided.
#[derive(Clone, Debug, PartialEq)]
pub struct Decisions<S, P> {
    /// Speakers to name.
    pub named: Vec<Naming<S, P>>,
    /// `(speaker, person, score)` to ask about.
    pub suggested: Vec<(S, P, f32)>,
}

/// Applies the three rules (see the module docs).
pub fn decide<S: Copy + Ord, P: Copy + Ord>(
    ballot: &Ballot<S, P>,
    thresholds: &IdentifyThresholds,
) -> Decisions<S, P> {
    let mut named = by_voice(ballot, thresholds);
    named.extend(by_elimination(&ballot.roster, &named));
    let suggested = suggestions(ballot, &named, thresholds.suggest);
    Decisions { named, suggested }
}

/// Rule 1: unique, mutual-best matches above the attendee or stranger bar.
fn by_voice<S: Copy + Ord, P: Copy + Ord>(
    ballot: &Ballot<S, P>,
    thresholds: &IdentifyThresholds,
) -> Vec<Naming<S, P>> {
    let rule = MatchRule {
        min_score: thresholds.auto_assign,
        min_margin: thresholds.margin,
    };
    pick_unique(&ballot.scores, rule)
        .into_iter()
        .filter(|pair| {
            ballot.attendees.contains(&pair.person)
                || pair.score >= thresholds.auto_assign_non_attendee
        })
        .map(|pair| Naming {
            speaker: pair.speaker,
            person: pair.person,
            by: NamedBy::Voice(pair.score),
        })
        .collect()
}

/// Rule 2: the last speaker is the last expected attendee.
fn by_elimination<S: Copy + Ord, P: Copy + Ord>(
    roster: &Roster<S, P>,
    named: &[Naming<S, P>],
) -> Option<Naming<S, P>> {
    let mut paired = roster.named.clone();
    paired.extend(named.iter().map(|naming| (naming.speaker, naming.person)));
    let (speaker, person) = complete_by_elimination(&roster.speakers, &roster.expected, &paired)?;
    Some(Naming {
        speaker,
        person,
        by: NamedBy::Elimination,
    })
}

/// Rule 3: each remaining person suggested once, best resemblance first.
fn suggestions<S: Copy + Ord, P: Copy + Ord>(
    ballot: &Ballot<S, P>,
    named: &[Naming<S, P>],
    floor: f32,
) -> Vec<(S, P, f32)> {
    let mut taken_speakers: Vec<S> = named.iter().map(|naming| naming.speaker).collect();
    let mut taken_people: Vec<P> = named.iter().map(|naming| naming.person).collect();
    taken_speakers.extend(ballot.roster.named.iter().map(|(speaker, _)| *speaker));
    taken_people.extend(ballot.roster.named.iter().map(|(_, person)| *person));
    let mut open: Vec<(S, P, f32)> = ballot
        .scores
        .iter()
        .copied()
        .filter(|(_, _, score)| *score >= floor)
        .collect();
    open.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    let mut suggested = Vec::new();
    for (speaker, person, score) in open {
        if taken_speakers.contains(&speaker) || taken_people.contains(&person) {
            continue;
        }
        taken_speakers.push(speaker);
        taken_people.push(person);
        suggested.push((speaker, person, score));
    }
    suggested
}

#[cfg(test)]
#[path = "decide_tests.rs"]
mod tests;
