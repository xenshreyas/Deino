//! [`CommentService`] over the real tables, including TRN-06's acceptance:
//! a thread survives transcript re-indexing and edits of other lines.

use std::sync::Arc;

use deino_db::{Database, test_db};
use deino_domain::{Channel, FakeClock, MeetingId, Millis, SegmentId};
use deino_testkit::{at, seed};

use super::*;
use crate::segments::{
    NewSegment, SegmentEditor, SegmentReader, SegmentWriter, SqliteSegmentRepo, TextEdit,
};

const MEETING: &str = "meeting-a";

struct Harness {
    service: CommentService,
    segments: SqliteSegmentRepo,
    database: Database,
    lines: Vec<NewSegment>,
}

fn line(index: u32, text: &str) -> NewSegment {
    let clock = FakeClock::new(at("2026-10-08T09:00:00Z"));
    NewSegment {
        id: SegmentId::generate(&clock),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::Mic,
        speaker_id: None,
        start: Millis(index * 1_000),
        end: Millis(index * 1_000 + 900),
        text: text.into(),
        words: Vec::new(),
    }
}

async fn harness() -> Harness {
    let database = test_db().await;
    seed::meeting(&database, MEETING, "Standup", at("2026-10-08T09:00:00Z")).await;
    seed::meeting(&database, "meeting-b", "Other", at("2026-10-08T09:00:00Z")).await;
    let segments = SqliteSegmentRepo::new(database.clone());
    let lines = vec![
        line(1, "We ship on Friday."),
        line(2, "The pricing page needs a rewrite."),
    ];
    segments.append(&lines).await.unwrap();
    let clock = Arc::new(FakeClock::new(at("2026-10-12T10:00:00Z")));
    let service = CommentService::new(Arc::new(SqliteCommentRepo::new(database.clone())), clock);
    Harness {
        service,
        segments,
        database,
        lines,
    }
}

fn on_line(segment: &NewSegment, quote: &str) -> NewThread {
    let start = segment.text.find(quote).unwrap();
    NewThread {
        meeting_id: MeetingId::new(MEETING),
        anchor: CommentAnchor {
            target: CommentTarget::Transcript,
            segment_id: Some(segment.id),
            start: u32::try_from(start).unwrap(),
            end: u32::try_from(start + quote.len()).unwrap(),
            quote: quote.into(),
        },
        body: "Who owns this?".into(),
    }
}

/// The text the thread's anchor points at in its line now.
async fn anchored_text(h: &Harness, thread: &CommentThread) -> Option<String> {
    let id = thread.anchor.segment_id?;
    let segment = h.segments.segment(&id).await.unwrap()?;
    let range = thread.anchor.start as usize..thread.anchor.end as usize;
    segment.text.get(range).map(str::to_owned)
}

#[tokio::test]
async fn a_thread_survives_reindexing_and_an_edit_of_another_line() {
    let h = harness().await;
    let thread = h
        .service
        .start(on_line(&h.lines[1], "pricing page"))
        .await
        .unwrap();

    // An edit of another line: its search documents are re-indexed and its
    // semantic windows dropped by the TRN-02 triggers.
    let edit = TextEdit {
        id: h.lines[0].id,
        text: "We ship on Monday instead.".into(),
    };
    h.segments
        .edit_many(&[edit], at("2026-10-12T10:05:00Z"))
        .await
        .unwrap();
    // A new earlier line shifts every row's position in the transcript.
    h.segments
        .append(&[line(0, "Morning, everyone.")])
        .await
        .unwrap();

    let listed = h.service.list(&MeetingId::new(MEETING)).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].anchor, thread.anchor);
    assert_eq!(
        anchored_text(&h, &listed[0]).await.as_deref(),
        Some("pricing page")
    );
    assert_eq!(h.segments.count(&MeetingId::new(MEETING)).await.unwrap(), 3);
}

#[tokio::test]
async fn a_retranscription_detaches_the_thread_but_keeps_it() {
    let h = harness().await;
    h.service
        .start(on_line(&h.lines[1], "rewrite"))
        .await
        .unwrap();

    h.segments
        .replace(&MeetingId::new(MEETING), &[line(1, "New words.")])
        .await
        .unwrap();

    let listed = h.service.list(&MeetingId::new(MEETING)).await.unwrap();
    assert_eq!(listed[0].anchor.segment_id, None);
    assert_eq!(listed[0].anchor.quote, "rewrite");
    assert_eq!(listed[0].comments.len(), 1);
}

#[tokio::test]
async fn replies_resolve_and_deleting_the_last_comment_drops_the_thread() {
    let h = harness().await;
    let thread = h
        .service
        .start(on_line(&h.lines[0], "Friday"))
        .await
        .unwrap();

    let reply = h.service.reply(thread.id, "  Maya does. ").await.unwrap();
    h.service.edit(reply.id, "Maya, by Thursday").await.unwrap();
    h.service.set_resolved(&thread.id, true).await.unwrap();
    let listed = h.service.list(&MeetingId::new(MEETING)).await.unwrap();
    let bodies: Vec<&str> = listed[0]
        .comments
        .iter()
        .map(|comment| comment.body.as_str())
        .collect();
    assert_eq!(bodies, ["Who owns this?", "Maya, by Thursday"]);
    assert!(listed[0].resolved_at.is_some());

    h.service.set_resolved(&thread.id, false).await.unwrap();
    h.service.delete(&thread.comments[0].id).await.unwrap();
    assert_eq!(
        h.service
            .list(&MeetingId::new(MEETING))
            .await
            .unwrap()
            .len(),
        1
    );
    h.service.delete(&reply.id).await.unwrap();
    assert!(
        h.service
            .list(&MeetingId::new(MEETING))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        h.service.reply(thread.id, "late").await,
        Err(CommentError::NotFound)
    ));
}

#[tokio::test]
async fn notes_threads_take_no_line_and_bad_anchors_are_refused() {
    let h = harness().await;
    let notes = NewThread {
        meeting_id: MeetingId::new(MEETING),
        anchor: CommentAnchor {
            target: CommentTarget::Notes,
            segment_id: None,
            start: 4,
            end: 9,
            quote: "plan".into(),
        },
        body: "Check".into(),
    };
    h.service.start(notes.clone()).await.unwrap();

    let mut empty = notes.clone();
    empty.anchor.end = 4;
    let mut other_meeting = on_line(&h.lines[0], "Friday");
    other_meeting.meeting_id = MeetingId::new("meeting-b");
    let blank = NewThread {
        body: "  ".into(),
        ..notes
    };

    assert!(matches!(
        h.service.start(empty).await,
        Err(CommentError::InvalidAnchor)
    ));
    assert!(matches!(
        h.service.start(other_meeting).await,
        Err(CommentError::InvalidAnchor)
    ));
    assert!(matches!(
        h.service.start(blank).await,
        Err(CommentError::EmptyBody)
    ));
}

#[tokio::test]
async fn a_trashed_meetings_threads_are_hidden() {
    let h = harness().await;
    let thread = h
        .service
        .start(on_line(&h.lines[0], "Friday"))
        .await
        .unwrap();

    sqlx::query("UPDATE meetings SET deleted_at = '2026-10-12T11:00:00Z' WHERE id = ?1")
        .bind(MEETING)
        .execute(h.database.writer())
        .await
        .unwrap();

    assert!(
        h.service
            .list(&MeetingId::new(MEETING))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        h.service.reply(thread.id, "hidden").await,
        Err(CommentError::NotFound)
    ));
}
