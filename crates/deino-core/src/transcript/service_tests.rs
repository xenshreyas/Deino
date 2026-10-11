//! The transcript read over fakes: order, hidden segments and speaker names.

use std::sync::Arc;

use deino_domain::{
    Channel, ErrorClass, ErrorCode, FakeClock, MeetingId, Millis, PaletteColor, PersonId,
    SegmentId, SpeakerId,
};
use deino_testkit::at;

use super::*;
use crate::people::{FakePeopleRepo, Person, PersonRepo};
use crate::segments::{FakeSegmentRepo, NewSegment, SegmentWriter, Speaker, SpeakerRepo};

const MEETING: &str = "meeting-a";

fn clock() -> FakeClock {
    FakeClock::new(at("2026-10-08T09:00:00Z"))
}

fn segment(start: u32, text: &str, speaker: Option<SpeakerId>) -> NewSegment {
    NewSegment {
        id: SegmentId::generate(&clock()),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: speaker,
        start: Millis(start),
        end: Millis(start + 900),
        text: text.into(),
        words: Vec::new(),
    }
}

fn speaker(label: &str, person: Option<PersonId>) -> Speaker {
    Speaker {
        id: SpeakerId::generate(&clock()),
        meeting_id: MeetingId::new(MEETING),
        label: label.into(),
        person_id: person,
        is_me: false,
        color: PaletteColor::SECOND,
    }
}

fn person(name: &str, merged_into: Option<PersonId>) -> Person {
    let created_at = at("2026-10-08T09:00:00Z");
    Person {
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
        merged_into,
        created_at,
        updated_at: created_at,
    }
}

struct Fixture {
    segments: Arc<FakeSegmentRepo>,
    people: Arc<FakePeopleRepo>,
    service: TranscriptService,
}

fn fixture() -> Fixture {
    let segments = Arc::new(FakeSegmentRepo::default());
    let people = Arc::new(FakePeopleRepo::default());
    let service = TranscriptService::new(TranscriptPorts {
        segments: segments.clone(),
        speakers: segments.clone(),
        people: people.clone(),
    });
    Fixture {
        segments,
        people,
        service,
    }
}

async fn names(fixture: &Fixture) -> Vec<(String, Option<PersonId>)> {
    let transcript = fixture
        .service
        .transcript(&MeetingId::new(MEETING))
        .await
        .unwrap();
    transcript
        .speakers
        .into_iter()
        .map(|speaker| (speaker.name, speaker.person_id))
        .collect()
}

#[tokio::test]
async fn segments_come_in_spoken_order_without_deleted_ones() {
    let fixture = fixture();
    let hidden = segment(500, "deleted", None);
    fixture
        .segments
        .append(&[segment(2_000, "later", None), hidden.clone()])
        .await
        .unwrap();
    fixture
        .segments
        .append(&[segment(0, "first", None)])
        .await
        .unwrap();
    fixture
        .segments
        .set_deleted(&hidden.id, true)
        .await
        .unwrap();

    let transcript = fixture
        .service
        .transcript(&MeetingId::new(MEETING))
        .await
        .unwrap();

    let texts: Vec<&str> = transcript
        .segments
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(texts, ["first", "later"]);
}

#[tokio::test]
async fn an_unidentified_speaker_shows_its_label() {
    let fixture = fixture();
    let them = speaker("Speaker 2", None);
    fixture.segments.add_speaker(&them).await.unwrap();

    assert_eq!(names(&fixture).await, [("Speaker 2".to_owned(), None)]);
}

#[tokio::test]
async fn an_identified_speaker_shows_the_persons_name() {
    let fixture = fixture();
    let priya = person("Priya Shah", None);
    fixture.people.create(&priya).await.unwrap();
    fixture
        .segments
        .add_speaker(&speaker("Speaker 1", Some(priya.id)))
        .await
        .unwrap();

    assert_eq!(
        names(&fixture).await,
        [("Priya Shah".to_owned(), Some(priya.id))]
    );
}

#[tokio::test]
async fn a_merged_person_shows_the_person_it_was_merged_into() {
    let fixture = fixture();
    let kept = person("Priya Shah", None);
    let merged = person("Priya", Some(kept.id));
    fixture.people.create(&kept).await.unwrap();
    fixture.people.create(&merged).await.unwrap();
    fixture
        .segments
        .add_speaker(&speaker("Speaker 1", Some(merged.id)))
        .await
        .unwrap();

    assert_eq!(
        names(&fixture).await,
        [("Priya Shah".to_owned(), Some(kept.id))]
    );
}

#[tokio::test]
async fn a_missing_person_falls_back_to_the_label() {
    let fixture = fixture();
    let gone = PersonId::generate(&clock());
    fixture
        .segments
        .add_speaker(&speaker("Speaker 3", Some(gone)))
        .await
        .unwrap();

    assert_eq!(names(&fixture).await, [("Speaker 3".to_owned(), None)]);
}

#[tokio::test]
async fn an_unknown_meeting_has_an_empty_transcript() {
    let fixture = fixture();

    let transcript = fixture
        .service
        .transcript(&MeetingId::new("meeting-none"))
        .await
        .unwrap();

    assert!(transcript.segments.is_empty());
    assert!(transcript.speakers.is_empty());
}

#[test]
fn storage_failures_are_internal() {
    let error = TranscriptError::Storage(deino_db::DbError::Query(sqlx::Error::RowNotFound));

    assert_eq!(error.code(), ErrorCode::Internal);
}
