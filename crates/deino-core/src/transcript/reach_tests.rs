//! TRN-02's acceptance over the real schema: a deleted line, and the words
//! an edit replaced, never reach the transcript view, the notes engine's
//! input, an export in any text format, the keyword index, or the `deino`
//! CLI and MCP server's read layer (`query::Library`: transcripts and
//! search). Semantic search is checked in `search::transcript_edit_tests`.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{Channel, FakeClock, MeetingId, Millis, SegmentId};
use deino_export::{ExportFormat, RenderOptions, render};
use deino_testkit::{at, seed};

use super::*;
use crate::enhance::MaterialSources;
use crate::export::{ExportAssembler, ExportPorts, FixedZone};
use crate::query::{Library, ReadContext, SearchQuery, TranscriptWindow};
use crate::segments::{NewSegment, Page, SegmentReader, SegmentWriter, SqliteSegmentRepo};

const MEETING: &str = "meeting-standup";
const SECRET: &str = "zebra-4471";
const REPLACED: &str = "Wednesday";

/// Ignores re-index requests: the triggers already did what reads need.
struct NoReindex;

impl TranscriptIndexer for NoReindex {
    fn changed(&self, _meetings: &[MeetingId]) {}
}

/// A standup with three lines; the first is deleted and the second edited.
async fn edited_standup() -> Database {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at("2026-10-08T09:00:00Z")).await;
    let repo = Arc::new(SqliteSegmentRepo::new(database.clone()));
    let clock = Arc::new(FakeClock::new(at("2026-10-08T10:00:00Z")));
    let line = |start: u32, text: &str| NewSegment {
        id: SegmentId::generate(&*clock),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::Mic,
        speaker_id: None,
        start: Millis(start),
        end: Millis(start + 900),
        text: text.into(),
        words: Vec::new(),
    };
    let lines = [
        line(0, &format!("The door code is {SECRET}.")),
        line(1_000, &format!("We ship on {REPLACED}.")),
        line(2_000, "Thanks, everyone."),
    ];
    repo.append(&lines).await.unwrap();
    let service = TranscriptEditService::new(TranscriptEditDeps {
        editor: repo.clone(),
        writer: repo,
        indexer: Arc::new(NoReindex),
        clock,
    });
    service.delete(&lines[0].id).await.unwrap();
    service
        .edit(&lines[1].id, "We ship on Friday.")
        .await
        .unwrap();
    database
}

/// Fails if `text` holds the deleted or the replaced words.
fn assert_clean(text: &str, place: &str) {
    assert!(!text.contains(SECRET), "{place} holds the deleted line");
    assert!(!text.contains(REPLACED), "{place} holds the replaced words");
}

#[tokio::test]
async fn the_transcript_and_the_cli_line_store_leave_them_out() {
    let database = edited_standup().await;
    let meeting = MeetingId::new(MEETING);
    let reader = SqliteSegmentRepo::new(database.clone());

    let view = TranscriptService::new(Arc::new(reader.clone()));
    let transcript = view.transcript(&meeting).await.unwrap();
    let listed = reader.list(&meeting, Page::ALL).await.unwrap();

    assert_eq!(transcript.segments.len(), 2);
    assert_eq!(listed.len(), 2);
    for segment in &listed {
        assert_clean(&segment.text, "the segment store");
    }
}

#[tokio::test]
async fn the_notes_engine_never_sees_them() {
    let database = edited_standup().await;

    let material = MaterialSources::sqlite(&database)
        .gather(&MeetingId::new(MEETING))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(material.transcript.len(), 2);
    for line in &material.transcript {
        assert_clean(&line.text, "the notes engine's input");
    }
    assert!(material.transcript_edited_at.is_some());
}

#[tokio::test]
async fn no_export_format_contains_them() {
    let database = edited_standup().await;
    let ports = ExportPorts::sqlite(&database, Arc::new(FixedZone::utc()));

    let export = ExportAssembler::new(ports)
        .assemble(&MeetingId::new(MEETING))
        .await
        .unwrap();

    assert_eq!(export.transcript.len(), 2);
    let text_formats = [
        ExportFormat::Markdown,
        ExportFormat::Text,
        ExportFormat::Json,
        ExportFormat::Srt,
        ExportFormat::Vtt,
    ];
    for format in text_formats {
        let rendered = render(format, &export, &RenderOptions::default()).unwrap();
        let text = String::from_utf8(rendered.bytes).unwrap();
        assert_clean(&text, &format!("the {format:?} export"));
    }
}

#[tokio::test]
async fn keyword_search_documents_follow_the_edits() {
    let database = edited_standup().await;

    let documents: Vec<String> =
        sqlx::query_scalar("SELECT body FROM search_documents WHERE kind = 'segment'")
            .fetch_all(database.reader())
            .await
            .unwrap();

    assert_eq!(documents.len(), 2);
    for body in &documents {
        assert_clean(body, "the keyword index");
    }
}

#[tokio::test]
async fn the_cli_and_mcp_read_layer_never_returns_them() {
    let database = edited_standup().await;
    let library = Library::sqlite(
        &database,
        &ReadContext {
            zone: Arc::new(FixedZone::utc()),
            clock: Arc::new(FakeClock::new(at("2026-10-09T09:00:00Z"))),
            embedder: None,
        },
    );
    let meeting = MeetingId::new(MEETING);

    let page = library
        .transcripts
        .page(&meeting, TranscriptWindow::default())
        .await
        .unwrap();
    let mut found = Vec::new();
    for words in [SECRET, REPLACED, "door code", "ship"] {
        let query = SearchQuery {
            text: words.into(),
            meeting: None,
            limit: 0,
        };
        found.extend(library.search.search(&query).await.unwrap().hits);
    }

    assert_eq!(page.lines.len(), 2);
    for line in &page.lines {
        assert_clean(&line.text, "the CLI transcript");
    }
    assert!(!found.is_empty(), "the edited line is still found");
    for hit in &found {
        assert_clean(&hit.snippet, "a CLI search hit");
    }
}
