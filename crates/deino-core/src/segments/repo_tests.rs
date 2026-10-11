//! The segment ports' contract, run against the SQLite repository and the
//! fake, plus what only the real schema can check (indexes, constraints).

use deino_db::test_db;
use deino_domain::{Channel, FakeClock, MeetingId, Millis, PaletteColor, SegmentId, SpeakerId};
use deino_testkit::{at, seed};

use super::*;
use crate::testing::query_plan;

const MEETING: &str = "meeting-a";

fn segment(start: u32, text: &str) -> NewSegment {
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    NewSegment {
        id: SegmentId::generate(&clock),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::Mic,
        speaker_id: None,
        start: Millis(start),
        end: Millis(start + 900),
        text: text.into(),
        words: vec![WordTiming {
            text: text.into(),
            start: Millis(start),
            end: Millis(start + 400),
        }],
    }
}

async fn sqlite_repo() -> SqliteSegmentRepo {
    let database = test_db().await;
    for id in [MEETING, "meeting-b"] {
        seed::meeting(&database, id, "Standup", at("2026-10-08T09:00:00Z")).await;
    }
    SqliteSegmentRepo::new(database)
}

fn texts(segments: &[Segment]) -> Vec<&str> {
    segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect()
}

/// Batches append, list in time order across batches, and page.
async fn appended_batches_list_in_time_order(repo: &(impl SegmentReader + SegmentWriter)) {
    let meeting = MeetingId::new(MEETING);
    repo.append(&[segment(2_000, "third"), segment(0, "first")])
        .await
        .unwrap();
    repo.append(&[segment(1_000, "second")]).await.unwrap();

    let all = repo.list(&meeting, Page::ALL).await.unwrap();
    let page = repo
        .list(
            &meeting,
            Page {
                limit: 1,
                offset: 1,
            },
        )
        .await
        .unwrap();

    assert_eq!(texts(&all), ["first", "second", "third"]);
    assert_eq!(texts(&page), ["second"]);
    assert_eq!(repo.count(&meeting).await.unwrap(), 3);
    assert_eq!(all[0].words, segment(0, "first").words);
    assert_eq!(all[0].channel, Channel::Mic);
}

/// Deleted segments leave the transcript; edits are stamped.
async fn deleted_segments_are_hidden_and_edits_stamped(
    repo: &(impl SegmentReader + SegmentWriter),
) {
    let meeting = MeetingId::new(MEETING);
    let (kept, hidden) = (segment(0, "kept"), segment(1_000, "hidden"));
    repo.append(&[kept.clone(), hidden.clone()]).await.unwrap();

    assert!(repo.set_deleted(&hidden.id, true).await.unwrap());
    assert!(
        repo.edit_text(&kept.id, "fixed", at("2026-10-09T10:00:00Z"))
            .await
            .unwrap()
    );

    let listed = repo.list(&meeting, Page::ALL).await.unwrap();
    assert_eq!(texts(&listed), ["fixed"]);
    assert_eq!(listed[0].edited_at, Some(at("2026-10-09T10:00:00Z")));
    assert_eq!(repo.count(&meeting).await.unwrap(), 1);
    assert!(!repo.set_deleted(&segment(0, "x").id, true).await.unwrap());
}

/// Replacing swaps one meeting's transcript and leaves the others alone.
async fn replace_swaps_only_that_meetings_transcript(repo: &(impl SegmentReader + SegmentWriter)) {
    let mut other = segment(0, "other meeting");
    other.meeting_id = MeetingId::new("meeting-b");
    repo.append(&[segment(0, "old"), other]).await.unwrap();

    repo.replace(&MeetingId::new(MEETING), &[segment(500, "new")])
        .await
        .unwrap();

    let mine = repo
        .list(&MeetingId::new(MEETING), Page::ALL)
        .await
        .unwrap();
    let theirs = repo
        .list(&MeetingId::new("meeting-b"), Page::ALL)
        .await
        .unwrap();
    assert_eq!(texts(&mine), ["new"]);
    assert_eq!(texts(&theirs), ["other meeting"]);
}

#[tokio::test]
async fn sqlite_appended_batches_list_in_time_order() {
    appended_batches_list_in_time_order(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_appended_batches_list_in_time_order() {
    appended_batches_list_in_time_order(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn sqlite_deleted_segments_are_hidden_and_edits_stamped() {
    deleted_segments_are_hidden_and_edits_stamped(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_deleted_segments_are_hidden_and_edits_stamped() {
    deleted_segments_are_hidden_and_edits_stamped(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn sqlite_replace_swaps_only_that_meetings_transcript() {
    replace_swaps_only_that_meetings_transcript(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_replace_swaps_only_that_meetings_transcript() {
    replace_swaps_only_that_meetings_transcript(&FakeSegmentRepo::default()).await;
}

/// A live save inserts new rows, completes stored drafts and deletes discards.
async fn live_saves_complete_drafts_and_delete_discards(
    repo: &(impl SegmentReader + SegmentWriter),
) {
    let meeting = MeetingId::new(MEETING);
    let mut draft = segment(0, "so the plan");
    let echo = segment(2_000, "echo");
    repo.save_live(&[draft.clone(), echo.clone()], &[])
        .await
        .unwrap();

    draft.text = "so the plan is final".into();
    draft.end = Millis(2_400);
    repo.save_live(&[draft.clone(), segment(3_000, "next")], &[echo.id])
        .await
        .unwrap();

    let all = repo.list(&meeting, Page::ALL).await.unwrap();
    assert_eq!(texts(&all), ["so the plan is final", "next"]);
    assert_eq!((all[0].id, all[0].end), (draft.id, Millis(2_400)));
}

#[tokio::test]
async fn sqlite_live_saves_complete_drafts_and_delete_discards() {
    live_saves_complete_drafts_and_delete_discards(&sqlite_repo().await).await;
}

#[tokio::test]
async fn fake_live_saves_complete_drafts_and_delete_discards() {
    live_saves_complete_drafts_and_delete_discards(&FakeSegmentRepo::default()).await;
}

#[tokio::test]
async fn a_failing_row_rolls_back_its_whole_batch() {
    let repo = sqlite_repo().await;
    let mut orphan = segment(1_000, "orphan");
    orphan.meeting_id = MeetingId::new("meeting-missing");

    let result = repo.append(&[segment(0, "fine"), orphan]).await;

    assert!(result.is_err());
    assert_eq!(repo.count(&MeetingId::new(MEETING)).await.unwrap(), 0);
}

#[tokio::test]
async fn a_batch_publishes_one_change_signal() {
    let repo = sqlite_repo().await;
    let mut changes = repo.database.changes().subscribe();

    repo.append(&[segment(0, "a"), segment(1_000, "b"), segment(2_000, "c")])
        .await
        .unwrap();

    let signal = changes.try_recv().unwrap();
    // The search index follows in the same commit (ORG-08 triggers).
    assert_eq!(
        signal.tables,
        [
            deino_db::Table::TranscriptSegments,
            deino_db::Table::SearchDocuments
        ]
    );
    assert!(changes.try_recv().is_err(), "one commit, one signal");
}

#[tokio::test]
async fn the_schema_rejects_an_end_before_the_start() {
    let repo = sqlite_repo().await;
    let mut backwards = segment(1_000, "backwards");
    backwards.end = Millis(500);

    assert!(repo.append(&[backwards]).await.is_err());
}

#[tokio::test]
async fn every_channel_round_trips_through_the_schema() {
    let repo = sqlite_repo().await;
    for (index, channel) in Channel::ALL.iter().enumerate() {
        let mut row = segment(u32::try_from(index).unwrap() * 1_000, channel.as_str());
        row.channel = *channel;
        repo.append(&[row]).await.unwrap();
    }

    let listed = repo
        .list(&MeetingId::new(MEETING), Page::ALL)
        .await
        .unwrap();

    let channels: Vec<Channel> = listed.iter().map(|segment| segment.channel).collect();
    assert_eq!(channels, Channel::ALL);
}

#[tokio::test]
async fn a_segment_cannot_take_a_speaker_from_another_meeting() {
    let repo = sqlite_repo().await;
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    let speaker = Speaker {
        id: SpeakerId::generate(&clock),
        meeting_id: MeetingId::new("meeting-b"),
        label: "Speaker 1".into(),
        person_id: None,
        is_me: false,
        color: PaletteColor::cycle(0),
    };
    repo.add_speaker(&speaker).await.unwrap();
    let row = segment(0, "hello");
    repo.append(std::slice::from_ref(&row)).await.unwrap();

    assert!(repo.set_speaker(&row.id, Some(&speaker.id)).await.is_err());
    let mut tagged = segment(1_000, "tagged");
    tagged.speaker_id = Some(speaker.id);
    assert!(repo.append(&[tagged]).await.is_err());
}

#[tokio::test]
async fn the_transcript_is_read_through_its_index() {
    let repo = sqlite_repo().await;

    let plan = query_plan(&repo.database, include_str!("queries/list.sql")).await;

    assert_eq!(
        plan,
        [
            "SEARCH transcript_segments USING INDEX idx_transcript_segments_meeting_id_start_ms (meeting_id=?)"
        ]
    );
}
