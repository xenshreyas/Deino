//! The naming rules on scripted scores. Speakers are numbers, people are
//! letters: `a` is Alex, `m` is Maya, `p` is Priya.

use super::*;

fn thresholds() -> IdentifyThresholds {
    IdentifyThresholds::default()
}

/// A ballot where every candidate is an attendee and nothing else is known.
fn attended(scores: &[(u8, char, f32)]) -> Ballot<u8, char> {
    let mut attendees: Vec<char> = scores.iter().map(|(_, person, _)| *person).collect();
    attendees.dedup();
    Ballot {
        scores: scores.to_vec(),
        attendees,
        roster: Roster::default(),
    }
}

fn named(decisions: &Decisions<u8, char>) -> Vec<(u8, char)> {
    decisions
        .named
        .iter()
        .map(|naming| (naming.speaker, naming.person))
        .collect()
}

#[test]
fn a_clear_attendee_voice_is_named() {
    let decisions = decide(&attended(&[(1, 'a', 0.84), (1, 'm', 0.21)]), &thresholds());

    assert_eq!(named(&decisions), [(1, 'a')]);
    assert_eq!(decisions.named[0].by, NamedBy::Voice(0.84));
    assert!(decisions.suggested.is_empty());
}

#[test]
fn an_unknown_voice_stays_unmatched_and_unsuggested() {
    // The spike: a voice never enrolled scores 0.35–0.38 at best.
    let decisions = decide(&attended(&[(1, 'a', 0.38), (1, 'm', 0.35)]), &thresholds());

    assert!(decisions.named.is_empty());
    assert!(decisions.suggested.is_empty());
}

#[test]
fn a_contaminated_phantom_cluster_is_rejected_by_mutual_best() {
    // The spike's failure: speaker 2 is a phantom split half from Maya,
    // half from Alex, and scores 0.81 against Alex. Alex's real voice (1)
    // is closer, so Alex is not the phantom's mutual best.
    let scores = [
        (1, 'a', 0.90),
        (1, 'm', 0.15),
        (2, 'a', 0.81),
        (2, 'm', 0.52),
        (3, 'a', 0.18),
        (3, 'm', 0.88),
    ];
    let decisions = decide(&attended(&scores), &thresholds());

    assert_eq!(named(&decisions), [(1, 'a'), (3, 'm')]);
    assert!(decisions.suggested.is_empty(), "Alex and Maya are taken");
}

#[test]
fn two_clusters_too_close_to_one_person_are_rejected_by_the_margin() {
    // Alex cannot tell voices 1 and 2 apart (0.85 vs 0.81 < 0.08 apart):
    // neither is named, and the closer one is offered for confirmation.
    let decisions = decide(&attended(&[(1, 'a', 0.85), (2, 'a', 0.81)]), &thresholds());

    assert!(decisions.named.is_empty());
    assert_eq!(decisions.suggested, [(1, 'a', 0.85)]);
}

#[test]
fn a_voice_between_two_people_is_suggested_not_named() {
    let decisions = decide(&attended(&[(1, 'a', 0.75), (1, 'm', 0.70)]), &thresholds());

    assert!(decisions.named.is_empty());
    assert_eq!(decisions.suggested, [(1, 'a', 0.75)]);
}

#[test]
fn a_stranger_needs_a_clearer_voice_than_an_attendee() {
    let ballot = |attendees: Vec<char>| Ballot {
        scores: vec![(1, 'p', 0.74)],
        attendees,
        roster: Roster::default(),
    };

    assert_eq!(
        named(&decide(&ballot(vec!['p']), &thresholds())),
        [(1, 'p')]
    );
    let stranger = decide(&ballot(Vec::new()), &thresholds());
    assert!(stranger.named.is_empty());
    assert_eq!(stranger.suggested, [(1, 'p', 0.74)]);
    let clear = Ballot {
        scores: vec![(1, 'p', 0.80)],
        ..ballot(Vec::new())
    };
    assert_eq!(named(&decide(&clear, &thresholds())), [(1, 'p')]);
}

#[test]
fn suggestions_start_at_the_suggest_threshold() {
    let decisions = decide(&attended(&[(1, 'a', 0.60), (2, 'm', 0.50)]), &thresholds());

    assert!(decisions.named.is_empty());
    assert_eq!(decisions.suggested, [(1, 'a', 0.60)]);
}

#[test]
fn the_last_attendee_is_named_by_elimination() {
    let ballot = Ballot {
        scores: vec![(1, 'a', 0.86), (2, 'a', 0.12)],
        attendees: vec!['a', 'm'],
        roster: Roster {
            speakers: vec![1, 2],
            expected: vec!['a', 'm'],
            named: Vec::new(),
        },
    };

    let decisions = decide(&ballot, &thresholds());

    assert_eq!(named(&decisions), [(1, 'a'), (2, 'm')]);
    assert_eq!(decisions.named[1].by, NamedBy::Elimination);
}

#[test]
fn a_one_on_one_call_needs_no_voiceprint() {
    let ballot = Ballot {
        scores: Vec::new(),
        attendees: vec!['m'],
        roster: Roster {
            speakers: vec![1],
            expected: vec!['m'],
            named: Vec::new(),
        },
    };

    assert_eq!(named(&decide(&ballot, &thresholds())), [(1, 'm')]);
}

#[test]
fn elimination_never_guesses_when_the_counts_differ() {
    let ballot = Ballot {
        scores: vec![(1, 'a', 0.86)],
        attendees: vec!['a', 'm'],
        roster: Roster {
            speakers: vec![1, 2, 3],
            expected: vec!['a', 'm'],
            named: Vec::new(),
        },
    };

    assert_eq!(named(&decide(&ballot, &thresholds())), [(1, 'a')]);
}

#[test]
fn speakers_the_user_named_count_for_elimination_and_are_never_suggested() {
    let ballot = Ballot {
        scores: vec![(2, 'a', 0.62)],
        attendees: vec!['a', 'm'],
        roster: Roster {
            speakers: vec![1, 2],
            expected: vec!['a', 'm'],
            named: vec![(1, 'a')],
        },
    };

    let decisions = decide(&ballot, &thresholds());

    assert_eq!(named(&decisions), [(2, 'm')]);
    assert!(decisions.suggested.is_empty());
}
