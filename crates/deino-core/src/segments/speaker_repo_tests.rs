//! The speaker port's contract, run against the SQLite repository and the
//! fake, plus what only the real schema can check.

use deino_db::test_db;
use deino_domain::{Channel, FakeClock, MeetingId, Millis, PaletteColor, SegmentId, SpeakerId};
use deino_testkit::{at, seed};

use super::*;
use crate::testing::query_plan;

const MEETING: &str = "meeting-a";

fn speaker(label: &str, index: usize) -> Speaker {
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    Speaker {
        id: SpeakerId::generate(&clock),
        meeting_id: MeetingId::new(MEETING),
        label: label.into(),
        person_id: None,
        is_me: index == 0,
        color: PaletteColor::cycle(index),
    }
}

async fn sqlite_repo() -> SqliteSegmentRepo {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at("2026-10-08T09:00:00Z")).await;
    SqliteSegmentRepo::new(database)
}

fn labels(speakers: &[Speaker]) -> Vec<&str> {
    speakers
        .iter()
        .map(|speaker| speaker.label.as_str())
        .collect()
}

/// Speakers list by label, and renames stick.
async fn speakers_list_by_label(repo: &impl SpeakerRepo) {
    let (me, them) = (speaker("Speaker 2", 0), speaker("Speaker 1", 1));
    repo.add_speaker(&me).await.unwrap();
    repo.add_speaker(&them).await.unwrap();

    assert!(repo.rename_speaker(&me.id, "Alex").await.unwrap());

    let listed = repo.speakers(&MeetingId::new(MEETING)).await.unwrap();
    assert_eq!(labels(&listed), ["Alex", "Speaker 1"]);
    assert!(listed[0].is_me);
    assert_eq!(listed[1].color, PaletteColor::cycle(1));
    assert!(
        !repo
            .rename_speaker(&speaker("x", 2).id, "Nobody")
            .await
            .unwrap()
    );
}

/// Deleting a speaker keeps its segments, without the speaker.
async fn deleting_a_speaker_keeps_its_segments(
    repo: &(impl SpeakerRepo + SegmentReader + SegmentWriter),
) {
    let voice = speaker("Speaker 1", 0);
    repo.add_speaker(&voice).await.unwrap();
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    let said = NewSegment {
        id: SegmentId::generate(&clock),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: Some(voice.id),
        start: Millis(0),
        end: Millis(1_000),
        text: "hello".into(),
        words: Vec::new(),
    };
    repo.append(&[said]).await.unwrap();

    assert!(repo.remove_speaker(&voice.id).await.unwrap());

    let segments = repo
        .list(&MeetingId::new(MEETING), Page::ALL)
        .await
        .unwrap();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].speaker_id, None);
}

#[tokio::test]
async fn sqlite_speakers_list_by_label() {
    speakers_list_by_label(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_speakers_list_by_label() {
    speakers_list_by_label(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn sqlite_deleting_a_speaker_keeps_its_segments() {
    deleting_a_speaker_keeps_its_segments(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_deleting_a_speaker_keeps_its_segments() {
    deleting_a_speaker_keeps_its_segments(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn labels_are_unique_within_a_meeting() {
    let repo = sqlite_repo().await;
    repo.add_speaker(&speaker("Speaker 1", 0)).await.unwrap();

    assert!(repo.add_speaker(&speaker("Speaker 1", 1)).await.is_err());
}

#[tokio::test]
async fn assigning_an_unknown_person_is_rejected() {
    let repo = sqlite_repo().await;
    let voice = speaker("Speaker 1", 0);
    repo.add_speaker(&voice).await.unwrap();
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));

    let stranger = deino_domain::PersonId::generate(&clock);

    assert!(
        repo.assign_person(&voice.id, Some(&stranger))
            .await
            .is_err()
    );
    assert!(repo.assign_person(&voice.id, None).await.unwrap());
}

#[tokio::test]
async fn speakers_are_listed_through_the_meeting_label_index() {
    let repo = sqlite_repo().await;

    let plan = query_plan(&repo.database, include_str!("queries/speakers.sql")).await;

    assert_eq!(
        plan,
        ["SEARCH speakers USING INDEX sqlite_autoindex_speakers_2 (meeting_id=?)"]
    );
}
