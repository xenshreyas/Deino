//! The transcript read over the fake segment repository.

use std::sync::Arc;

use deino_domain::{
    Channel, ErrorClass, ErrorCode, FakeClock, MeetingId, Millis, SegmentId, SpeakerId,
};
use deino_testkit::at;

use super::*;
use crate::segments::{FakeSegmentRepo, NewSegment, SegmentWriter};

const MEETING: &str = "meeting-a";

fn segment(start: u32, text: &str) -> NewSegment {
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    NewSegment {
        id: SegmentId::generate(&clock),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: Some(SpeakerId::generate(&clock)),
        start: Millis(start),
        end: Millis(start + 900),
        text: text.into(),
        words: Vec::new(),
    }
}

fn service() -> (Arc<FakeSegmentRepo>, TranscriptService) {
    let repo = Arc::new(FakeSegmentRepo::default());
    (repo.clone(), TranscriptService::new(repo))
}

#[tokio::test]
async fn segments_come_in_spoken_order_with_their_speakers_and_without_deleted_ones() {
    let (repo, service) = service();
    let hidden = segment(500, "deleted");
    let later = segment(2_000, "later");
    repo.append(&[later.clone(), hidden.clone()]).await.unwrap();
    repo.append(&[segment(0, "first")]).await.unwrap();
    repo.set_deleted(&hidden.id, true).await.unwrap();

    let transcript = service.transcript(&MeetingId::new(MEETING)).await.unwrap();

    let texts: Vec<&str> = transcript
        .segments
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(texts, ["first", "later"]);
    assert_eq!(transcript.segments[1].speaker_id, later.speaker_id);
}

#[tokio::test]
async fn an_unknown_meeting_has_an_empty_transcript() {
    let (_, service) = service();

    let transcript = service
        .transcript(&MeetingId::new("meeting-none"))
        .await
        .unwrap();

    assert!(transcript.segments.is_empty());
}

#[test]
fn storage_failures_are_internal() {
    let error = TranscriptError::Storage(deino_db::DbError::Query(sqlx::Error::RowNotFound));

    assert_eq!(error.code(), ErrorCode::Internal);
}
