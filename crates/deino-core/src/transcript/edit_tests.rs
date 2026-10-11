//! The transcript edits over the fake segment repository and a recording
//! indexer.

use std::sync::{Arc, Mutex};

use deino_domain::{Channel, ErrorClass, ErrorCode, FakeClock, MeetingId, Millis, SegmentId};
use deino_testkit::at;

use super::*;
use crate::segments::{FakeSegmentRepo, NewSegment, Page, SegmentReader, SegmentWriter};
use crate::testing::lock;

const STANDUP: &str = "meeting-standup";
const PLANNING: &str = "meeting-planning";

/// Remembers which meetings were queued for re-indexing.
#[derive(Default)]
struct RecordingIndexer(Mutex<Vec<MeetingId>>);

impl TranscriptIndexer for RecordingIndexer {
    fn changed(&self, meetings: &[MeetingId]) {
        lock(&self.0).extend_from_slice(meetings);
    }
}

struct Harness {
    repo: Arc<FakeSegmentRepo>,
    indexer: Arc<RecordingIndexer>,
    service: TranscriptEditService,
    clock: Arc<FakeClock>,
}

impl Harness {
    fn new() -> Self {
        let repo = Arc::new(FakeSegmentRepo::default());
        let indexer = Arc::new(RecordingIndexer::default());
        let clock = Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z")));
        let service = TranscriptEditService::new(TranscriptEditDeps {
            editor: repo.clone(),
            writer: repo.clone(),
            indexer: indexer.clone(),
            clock: clock.clone(),
        });
        Self {
            repo,
            indexer,
            service,
            clock,
        }
    }

    async fn line(&self, meeting: &str, start: u32, text: &str) -> SegmentId {
        let id = SegmentId::generate(&*self.clock);
        let segment = NewSegment {
            id,
            meeting_id: MeetingId::new(meeting),
            channel: Channel::System,
            speaker_id: None,
            start: Millis(start),
            end: Millis(start + 900),
            text: text.into(),
            words: Vec::new(),
        };
        self.repo.append(&[segment]).await.unwrap();
        id
    }

    async fn texts(&self, meeting: &str) -> Vec<String> {
        let segments = self.repo.list(&MeetingId::new(meeting), Page::ALL).await;
        segments.unwrap().into_iter().map(|s| s.text).collect()
    }

    fn reindexed(&self) -> Vec<String> {
        lock(&self.indexer.0)
            .iter()
            .map(ToString::to_string)
            .collect()
    }
}

fn request(find: &str, scope: ReplaceScope) -> ReplaceRequest {
    ReplaceRequest {
        find: find.into(),
        scope,
        match_case: false,
    }
}

fn standup() -> ReplaceScope {
    ReplaceScope::Meeting {
        meeting_id: MeetingId::new(STANDUP),
    }
}

#[tokio::test]
async fn an_edit_stamps_the_time_keeps_the_first_original_and_reindexes() {
    let harness = Harness::new();
    let id = harness.line(STANDUP, 0, "We ship on Monday.").await;

    harness
        .service
        .edit(&id, " We ship on Tuesday. ")
        .await
        .unwrap();
    let edited = harness
        .service
        .edit(&id, "We ship Wednesday.")
        .await
        .unwrap();

    assert_eq!(edited.text, "We ship Wednesday.");
    assert_eq!(edited.original_text.as_deref(), Some("We ship on Monday."));
    assert_eq!(edited.edited_at, Some(at("2026-10-08T10:00:00Z")));
    assert_eq!(harness.reindexed(), [STANDUP, STANDUP]);
}

#[tokio::test]
async fn the_same_text_again_changes_nothing() {
    let harness = Harness::new();
    let id = harness.line(STANDUP, 0, "Same.").await;

    let unchanged = harness.service.edit(&id, "Same.").await.unwrap();

    assert_eq!(unchanged.edited_at, None);
    assert!(harness.reindexed().is_empty());
}

#[tokio::test]
async fn blank_long_missing_and_deleted_lines_are_refused_before_writing() {
    let harness = Harness::new();
    let id = harness.line(STANDUP, 0, "Keep me.").await;
    let long = "x".repeat(MAX_LINE_CHARS + 1);
    let missing = SegmentId::generate(&*harness.clock);

    let blank = harness.service.edit(&id, "   ").await.unwrap_err();
    let too_long = harness.service.edit(&id, &long).await.unwrap_err();
    let unknown = harness.service.edit(&missing, "text").await.unwrap_err();
    harness.service.delete(&id).await.unwrap();
    let deleted = harness.service.edit(&id, "text").await.unwrap_err();

    assert_eq!(blank.code(), ErrorCode::Invalid);
    assert_eq!(too_long.code(), ErrorCode::Invalid);
    assert_eq!(unknown.code(), ErrorCode::NotFound);
    assert_eq!(deleted.code(), ErrorCode::NotFound);
}

#[tokio::test]
async fn a_deleted_line_leaves_the_transcript_and_restore_brings_it_back() {
    let harness = Harness::new();
    let secret = harness.line(STANDUP, 0, "The password is hunter2.").await;
    harness.line(STANDUP, 1_000, "Moving on.").await;

    harness.service.delete(&secret).await.unwrap();
    harness.service.delete(&secret).await.unwrap();
    assert_eq!(harness.texts(STANDUP).await, ["Moving on."]);

    harness.service.restore(&secret).await.unwrap();
    assert_eq!(harness.texts(STANDUP).await.len(), 2);
    // Deleting twice is one change: two re-indexes, delete and restore.
    assert_eq!(harness.reindexed(), [STANDUP, STANDUP]);
}

#[tokio::test]
async fn the_preview_counts_what_replace_then_changes_in_one_meeting() {
    let harness = Harness::new();
    harness.line(STANDUP, 0, "Dino and dino.").await;
    harness.line(STANDUP, 1_000, "No match.").await;
    harness.line(PLANNING, 0, "dino elsewhere").await;
    harness
        .repo
        .title_meeting(&MeetingId::new(STANDUP), "Standup");

    let preview = harness
        .service
        .preview(&request("dino", standup()))
        .await
        .unwrap();
    let outcome = harness
        .service
        .replace(&request("dino", standup()), "Deino")
        .await
        .unwrap();

    assert_eq!((preview.matches, preview.segments), (2, 1));
    assert_eq!(preview.meetings[0].title, "Standup");
    assert_eq!(
        (outcome.matches, outcome.segments, outcome.meetings),
        (2, 1, 1)
    );
    assert_eq!(harness.texts(STANDUP).await[0], "Deino and Deino.");
    assert_eq!(harness.texts(PLANNING).await, ["dino elsewhere"]);
    assert_eq!(harness.reindexed(), [STANDUP]);
}

#[tokio::test]
async fn replace_across_every_meeting_respects_match_case() {
    let harness = Harness::new();
    harness.line(STANDUP, 0, "Deino ships.").await;
    harness.line(PLANNING, 0, "deino ships.").await;
    let exact = ReplaceRequest {
        match_case: true,
        ..request("Deino", ReplaceScope::All)
    };

    let outcome = harness.service.replace(&exact, "Dino").await.unwrap();

    assert_eq!(outcome.meetings, 1);
    assert_eq!(harness.texts(STANDUP).await, ["Dino ships."]);
    assert_eq!(harness.texts(PLANNING).await, ["deino ships."]);
}

#[tokio::test]
async fn finding_nothing_is_refused_and_no_match_writes_nothing() {
    let harness = Harness::new();
    harness.line(STANDUP, 0, "Hello.").await;

    let blank = harness.service.preview(&request("  ", standup())).await;
    let none = harness
        .service
        .replace(&request("absent", standup()), "x")
        .await;

    assert_eq!(blank.unwrap_err().code(), ErrorCode::Invalid);
    assert_eq!(none.unwrap(), ReplaceOutcome::default());
    assert!(harness.reindexed().is_empty());
}
