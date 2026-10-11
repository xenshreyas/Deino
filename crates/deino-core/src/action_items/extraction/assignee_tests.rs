//! Names to people: exact, partial, by address, with typos, attendees
//! before speakers, and ambiguity left unassigned.

use deino_domain::PersonId;

use super::*;

fn id(n: u8) -> PersonId {
    format!("0199c6a0-0000-7000-8000-0000000000{n:02}")
        .parse()
        .unwrap()
}

fn candidate(n: u8, name: &str, email: Option<&str>) -> Candidate {
    Candidate {
        person: id(n),
        name: name.into(),
        email: email.map(str::to_owned),
    }
}

/// Shreyas (the user), Maya Chen, Marcus Webb, Jordan Ellis and Maya Patel
/// attend; Owen Hart only spoke, as "Speaker 3".
fn roster() -> Roster {
    Roster {
        me: Some(id(1)),
        attendees: vec![
            candidate(1, "Shreyas Rao", Some("shreyas@deino.dev")),
            candidate(2, "Maya Chen", Some("maya@deino.dev")),
            candidate(
                3,
                "Marcus Webb",
                Some("marcus.webb@brightkite-logistics.com"),
            ),
            candidate(4, "Jordan Ellis", Some("jordan@jordanellis.dev")),
            candidate(5, "Maya Patel", None),
        ],
        speakers: vec![
            candidate(6, "Owen Hart", Some("owen@fernhollow.co")),
            candidate(6, "Speaker 3", None),
            candidate(2, "Maya Chen", Some("maya@deino.dev")),
        ],
    }
}

#[test]
fn the_user_is_me_in_any_casing_and_their_own_name() {
    let roster = roster();

    assert_eq!(roster.resolve("me"), Some(id(1)));
    assert_eq!(roster.resolve("Me"), Some(id(1)));
    assert_eq!(roster.resolve("Shreyas"), Some(id(1)));
    assert_eq!(roster.resolve("Shreyas Rao (me)"), Some(id(1)));
}

#[test]
fn nobody_and_the_team_stay_unassigned() {
    let roster = roster();

    for said in ["", "  ", "null", "Unassigned", "team", "We"] {
        assert_eq!(roster.resolve(said), None, "{said:?}");
    }
}

#[test]
fn whole_names_win_even_when_a_first_name_is_shared() {
    let roster = roster();

    assert_eq!(roster.resolve("Maya Chen"), Some(id(2)));
    assert_eq!(roster.resolve("maya patel"), Some(id(5)));
    // Two Mayas attend: a first name alone is ambiguous.
    assert_eq!(roster.resolve("Maya"), None);
}

#[test]
fn a_unique_first_or_last_name_is_enough() {
    let roster = roster();

    assert_eq!(roster.resolve("Marcus"), Some(id(3)));
    assert_eq!(roster.resolve("Webb"), Some(id(3)));
    assert_eq!(roster.resolve("Jordan"), Some(id(4)));
}

#[test]
fn transcription_typos_and_accents_still_match_one_person() {
    let roster = roster();

    assert_eq!(roster.resolve("Jordan Elis"), Some(id(4)));
    assert_eq!(roster.resolve("Markus"), Some(id(3)));
    assert_eq!(roster.resolve("Jördan Éllis"), Some(id(4)));
    assert_eq!(roster.resolve("Mara"), None);
}

#[test]
fn an_address_identifies_its_owner() {
    assert_eq!(roster().resolve("marcus.webb"), Some(id(3)));
}

#[test]
fn speakers_who_were_not_invited_are_found_by_name_or_label() {
    let roster = roster();

    assert_eq!(roster.resolve("Owen"), Some(id(6)));
    assert_eq!(roster.resolve("Speaker 3"), Some(id(6)));
    assert_eq!(roster.resolve("Speaker 4"), None);
}

#[test]
fn attendees_are_searched_before_speakers() {
    let mut roster = roster();
    roster.speakers.push(candidate(7, "Marcus Webber", None));

    assert_eq!(roster.resolve("Marcus"), Some(id(3)));
}

#[test]
fn edit_distance_counts_insertions_deletions_and_substitutions() {
    assert_eq!(edit_distance("ellis", "elis"), 1);
    assert_eq!(edit_distance("kitten", "sitting"), 3);
    assert_eq!(edit_distance("", "abc"), 3);
    assert_eq!(edit_distance("same", "same"), 0);
}
