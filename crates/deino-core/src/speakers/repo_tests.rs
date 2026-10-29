//! What only the real schema and transactions can check.

use deino_db::{Database, test_db};
use deino_domain::{
    Channel, DisplayName, FakeClock, MeetingId, Millis, PaletteColor, PersonId, SegmentId,
    SpeakerId,
};
use deino_testkit::{at, seed};

use super::*;
use crate::people::{Person, PersonRepo, SqlitePeopleRepo};
use crate::segments::{
    NewSegment, Page, SegmentReader, SegmentWriter, Speaker, SpeakerRepo, SqliteSegmentRepo,
};
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
        ..Identification::default()
    };
    let second = Identification {
        named: Vec::new(),
        suggested: vec![suggest(maya)],
        ..Identification::default()
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

/// A system-track segment of `speaker` in the meeting.
async fn segment_of(segments: &SqliteSegmentRepo, speaker: &Speaker) -> SegmentId {
    let id = SegmentId::generate(&clock());
    let segment = NewSegment {
        id,
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: Some(speaker.id),
        start: Millis(0),
        end: Millis(1_000),
        text: "…".into(),
        words: Vec::new(),
    };
    segments.append(&[segment]).await.unwrap();
    id
}

/// A meeting with an unnamed speaker, a stray one with a segment, and one
/// the user named with a segment.
async fn seating_room(database: &Database) -> ([Speaker; 3], [SegmentId; 2]) {
    seed::meeting(database, MEETING, "Standup", at(T0)).await;
    let maya = person(&SqlitePeopleRepo::new(database.clone()), "Maya").await;
    let segments = SqliteSegmentRepo::new(database.clone());
    let (open, stray) = (speaker("Speaker 1", None), speaker("Speaker 2", None));
    let named = speaker("Speaker 3", Some(maya));
    for each in [&open, &stray, &named] {
        segments.add_speaker(each).await.unwrap();
    }
    let on_stray = segment_of(&segments, &stray).await;
    let on_named = segment_of(&segments, &named).await;
    ([open, stray, named], [on_stray, on_named])
}

#[tokio::test]
async fn seating_creates_labels_moves_and_empties_but_never_touches_named_speakers() {
    let database = test_db().await;
    let ([open, stray, named], [on_stray, on_named]) = seating_room(&database).await;
    let created = speaker("Cleo", None);
    let to_created = |segment| SegmentMove {
        segment,
        to: created.id,
    };
    let label = |text| DisplayName::parse(text).unwrap();
    let decided = Identification {
        seating: Seating {
            created: vec![created.clone()],
            labelled: vec![(open.id, label("Alex")), (named.id, label("Not Maya"))],
            moved: vec![to_created(on_stray), to_created(on_named)],
            emptied: vec![stray.id, named.id],
        },
        ..Identification::default()
    };

    let meeting = MeetingId::new(MEETING);
    let repo = SqliteNamingRepo::new(database.clone());
    repo.apply(&meeting, &decided).await.unwrap();

    let segments = SqliteSegmentRepo::new(database);
    let after = segments.speakers(&meeting).await.unwrap();
    let labels: Vec<&str> = after.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, ["Alex", "Cleo", "Speaker 3"]);
    let listed = segments.list(&meeting, Page::ALL).await.unwrap();
    let owner = |id| listed.iter().find(|s| s.id == id).unwrap().speaker_id;
    assert_eq!(owner(on_stray), Some(created.id));
    assert_eq!(owner(on_named), Some(named.id));
}

#[tokio::test]
async fn participants_read_speakers_and_speech_through_indexes() {
    let database = test_db().await;
    let plan = query_plan(&database, include_str!("queries/participants.sql")).await;
    let uses = |index: &str| plan.iter().any(|step| step.contains(index));
    assert!(uses("(meeting_id=?)"), "{plan:?}");
    assert!(uses("idx_transcript_segments_speaker_id"), "{plan:?}");
}

/// Gives `speaker` a stored voice of `speech_ms`, every value `value`.
async fn give_voice(database: &Database, speaker: &Speaker, value: f32, speech_ms: i64) {
    let centroid: Vec<u8> = std::iter::repeat_n(value.to_le_bytes(), 256)
        .flatten()
        .collect();
    sqlx::query(
        "UPDATE speakers SET centroid = ?1, embedding_model = 'm', speech_ms = ?2 WHERE id = ?3",
    )
    .bind(centroid)
    .bind(speech_ms)
    .bind(speaker.id)
    .execute(database.writer())
    .await
    .unwrap();
}

#[tokio::test]
async fn a_merge_keeps_the_voice_heard_for_longest() {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Interview", at(T0)).await;
    let segments = SqliteSegmentRepo::new(database.clone());
    let repo = SqliteNamingRepo::new(database.clone());
    let [voiced, named_only, quieter] = [
        speaker("Speaker 1", None),
        speaker("Mehrdad", None),
        speaker("Speaker 2", None),
    ];
    for s in [&voiced, &named_only, &quieter] {
        segments.add_speaker(s).await.unwrap();
    }
    give_voice(&database, &voiced, 0.5, 600_000).await;
    give_voice(&database, &quieter, 0.25, 30_000).await;
    let merge = |from: &Speaker, into: &Speaker| MergeSpeakers {
        meeting_id: MeetingId::new(MEETING),
        from: from.clone(),
        into: into.clone(),
        at: at(T0),
    };

    repo.merge(&merge(&voiced, &named_only)).await.unwrap();
    let kept = segments.voice(&named_only.id).await.unwrap().unwrap();
    assert_eq!(
        kept.speech,
        Millis(600_000),
        "a speaker without a voice takes the merged one's"
    );

    repo.merge(&merge(&quieter, &named_only)).await.unwrap();
    let kept = segments.voice(&named_only.id).await.unwrap().unwrap();
    assert_eq!(
        kept.speech,
        Millis(600_000),
        "a shorter voice never replaces a longer one"
    );
    assert!((kept.centroid[0] - 0.5).abs() < 1e-6);
}
