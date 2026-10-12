use deino_domain::{MeetingId, PersonId};
use deino_llm::brief::{BriefAnswer, BriefAnswerBullet};
use deino_testkit::at;

use super::*;

fn meeting(body: &str) -> Source {
    Source {
        citation: BriefCitation::Meeting {
            meeting_id: MeetingId::new("meeting-1"),
            title: "Kickoff".into(),
            at: at("2026-10-01T09:00:00Z"),
        },
        heading: "Meeting: Kickoff (Thursday 1 October 2026)".into(),
        body: body.into(),
    }
}

fn person() -> Source {
    Source {
        citation: BriefCitation::Person {
            person_id: "01926b2c-0000-7000-8000-000000000001"
                .parse::<PersonId>()
                .unwrap(),
            name: "Maya Chen".into(),
        },
        heading: "About Maya Chen".into(),
        body: "Leads infrastructure.".into(),
    }
}

fn invitation() -> Source {
    Source {
        citation: BriefCitation::Event,
        heading: "Invitation".into(),
        body: "Agenda: renewal.".into(),
    }
}

fn inputs(sources: Vec<Source>) -> BriefInputs {
    BriefInputs {
        event: "Title: Renewal".into(),
        sources,
    }
}

#[test]
fn an_invitation_alone_or_an_empty_meeting_is_not_enough() {
    assert!(!inputs(vec![invitation()]).is_sufficient());
    assert!(!inputs(vec![meeting("  "), invitation()]).is_sufficient());
    assert!(!inputs(vec![person(), invitation()]).is_sufficient());
}

#[test]
fn a_meeting_with_notes_or_two_other_facts_is_enough() {
    assert!(inputs(vec![meeting("Agreed on a pilot.")]).is_sufficient());
    assert!(inputs(vec![person(), person()]).is_sufficient());
}

#[test]
fn sources_are_numbered_per_kind_and_the_fingerprint_follows_the_text() {
    let rendered = inputs(vec![meeting("Pilot."), person(), invitation()]).render();

    let user = rendered.prompt.messages[0].text();
    assert!(user.contains("[m1] Meeting: Kickoff"));
    assert!(user.contains("[p1] About Maya Chen"));
    assert!(user.contains("[e1] Invitation"));
    assert_eq!(rendered.fingerprint.len(), 64);
    let again = inputs(vec![meeting("Pilot."), person(), invitation()]).render();
    assert_eq!(again.fingerprint, rendered.fingerprint);
    let changed = inputs(vec![meeting("Pilot, signed."), person(), invitation()]).render();
    assert_ne!(changed.fingerprint, rendered.fingerprint);
}

fn bullet(text: &str, sources: &[&str]) -> BriefAnswerBullet {
    BriefAnswerBullet {
        text: text.into(),
        sources: sources.iter().map(|&source| source.to_owned()).collect(),
    }
}

#[test]
fn references_resolve_once_each_and_unknown_ones_are_dropped() {
    let rendered = inputs(vec![meeting("Pilot."), person(), invitation()]).render();
    let refs = ["[m1]", "M1", "p1", "m2", "x1", "e1"].map(String::from);

    let citations = rendered.index.resolve(&refs);

    assert_eq!(citations.len(), MAX_CITATIONS);
    assert!(matches!(citations[0], BriefCitation::Meeting { .. }));
    assert!(matches!(citations[1], BriefCitation::Person { .. }));
    assert_eq!(citations[2], BriefCitation::Event);
}

#[test]
fn bullets_need_a_real_citation_and_there_must_be_two_to_four() {
    let rendered = inputs(vec![meeting("Pilot."), person()]).render();
    let answer = |bullets| BriefAnswer {
        insufficient: false,
        bullets,
    };

    let five: Vec<_> = (0..5)
        .map(|i| bullet(&format!("Fact {i}."), &["m1"]))
        .collect();
    assert_eq!(
        rendered.index.bullets(answer(five)).unwrap().len(),
        MAX_BULLETS
    );
    let one_real = vec![
        bullet("Real.", &["p1"]),
        bullet("Invented.", &["m7"]),
        bullet(" ", &["m1"]),
    ];
    assert_eq!(rendered.index.bullets(answer(one_real)), None);
    let flagged = BriefAnswer {
        insufficient: true,
        bullets: vec![bullet("A.", &["m1"]), bullet("B.", &["p1"])],
    };
    assert_eq!(rendered.index.bullets(flagged), None);
}
