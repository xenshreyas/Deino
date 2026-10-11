//! What only the real schema and transactions can check.

use deino_db::test_db;
use deino_domain::{FakeClock, MeetingId, PaletteColor, PersonId, SegmentId, SpeakerId};
use deino_testkit::{at, seed};

use super::*;
use crate::people::{Person, PersonRepo, SqlitePeopleRepo};
use crate::segments::{Speaker, SpeakerRepo, SqliteSegmentRepo};
use crate::testing::query_plan;

const MEETING: &str = "meeting-a";
const T0: &str = "2026-10-11T09:00:00Z";

fn clock() -> FakeClock {
    FakeClock::new(at(T0))
}

fn speaker(label: &str, person: Option<PersonId>) -> Speaker {
    Speaker {
        id: SpeakerId::generate(&clock()),
        meeting_id: MeetingId::new(MEETING),
        label: label.into(),
        person_id: person,
        is_me: false,
        color: PaletteColor::cycle(1),
    }
}

async fn person(people: &SqlitePeopleRepo, name: &str) -> PersonId {
    let at = at(T0);
    let person = Person {
        id: PersonId::generate(&clock()),
        name: name.into(),
        email: None,
        company_id: None,
        title: None,
        avatar_path: None,
        notes_md: String::new(),
        summary_md: None,
        summary_updated_at: None,
        is_me: false,
        merged_into: None,
        created_at: at,
        updated_at: at,
    };
    people.create(&person).await.unwrap();
    person.id
}

#[tokio::test]
async fn renaming_a_missing_segment_rolls_the_new_speaker_back() {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at(T0)).await;
    let repo = SqliteNamingRepo::new(database.clone());
    let target = SegmentTarget::New(speaker("Speaker 9", None));
    let meeting = MeetingId::new(MEETING);

    let moved = repo
        .name_segment(&meeting, &SegmentId::generate(&clock()), &target)
        .await
        .unwrap();

    assert!(!moved);
    let speakers = SqliteSegmentRepo::new(database)
        .speakers(&meeting)
        .await
        .unwrap();
    assert!(speakers.is_empty());
}

#[tokio::test]
async fn the_stage_never_renames_a_speaker_the_user_named_and_replaces_suggestions() {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at(T0)).await;
    let people = SqlitePeopleRepo::new(database.clone());
    let (alex, maya) = (person(&people, "Alex").await, person(&people, "Maya").await);
    let named = speaker("Speaker 1", Some(maya));
    let open = speaker("Speaker 2", None);
    let segments = SqliteSegmentRepo::new(database.clone());
    segments.add_speaker(&named).await.unwrap();
    segments.add_speaker(&open).await.unwrap();
    let repo = SqliteNamingRepo::new(database);
    let meeting = MeetingId::new(MEETING);
    let suggest = |person_id| SuggestedName {
        speaker_id: open.id,
        person_id,
        score: 0.6,
    };
    let first = Identification {
        named: vec![(named.id, alex)],
        suggested: vec![suggest(alex)],
    };
    let second = Identification {
        named: Vec::new(),
        suggested: vec![suggest(maya)],
    };

    repo.apply(&meeting, &first).await.unwrap();
    repo.apply(&meeting, &second).await.unwrap();

    let rows = repo.participants(&meeting).await.unwrap();
    let by_label = |label: &str| rows.iter().find(|row| row.speaker.label == label).unwrap();
    assert_eq!(by_label("Speaker 1").speaker.person_id, Some(maya));
    let suggestion = by_label("Speaker 2").suggestion.clone().unwrap();
    assert_eq!(suggestion.person.id, maya);
    assert!((suggestion.score - 0.6).abs() < 1e-6);
}

#[tokio::test]
async fn participants_read_speakers_and_speech_through_indexes() {
    let database = test_db().await;
    let plan = query_plan(&database, include_str!("queries/participants.sql")).await;
    let uses = |index: &str| plan.iter().any(|step| step.contains(index));
    assert!(uses("(meeting_id=?)"), "{plan:?}");
    assert!(uses("idx_transcript_segments_speaker_id"), "{plan:?}");
}
