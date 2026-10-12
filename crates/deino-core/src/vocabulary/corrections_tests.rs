//! Corrections and undo against the real schema.

use deino_db::test_db;
use deino_domain::{Channel, FakeClock, Millis};
use deino_testkit::{at, seed};

use super::*;
use crate::segments::{NewSegment, Page, SegmentReader, SegmentWriter, SqliteSegmentRepo};
use crate::testing::query_plan;

const MEETING: &str = "meeting-a";

fn word(text: &str, start: u32) -> WordTiming {
    WordTiming {
        text: text.into(),
        start: Millis(start),
        end: Millis(start + 300),
    }
}

fn segment(start: u32, text: &str) -> NewSegment {
    NewSegment {
        id: SegmentId::generate(&FakeClock::new(at("2026-10-11T09:00:00Z"))),
        meeting_id: MeetingId::new(MEETING),
        channel: Channel::System,
        speaker_id: None,
        start: Millis(start),
        end: Millis(start + 900),
        text: text.into(),
        words: vec![word(text, start)],
    }
}

fn correction(segment: &NewSegment, corrected: &str) -> Correction {
    Correction {
        segment_id: segment.id,
        original_text: segment.text.clone(),
        original_words: segment.words.clone(),
        corrected_text: corrected.into(),
        corrected_words: vec![word(corrected, segment.start.0)],
    }
}

struct Fixture {
    corrections: SqliteCorrectionRepo,
    segments: SqliteSegmentRepo,
    meeting: MeetingId,
}

impl Fixture {
    async fn new(texts: &[&str]) -> (Self, Vec<NewSegment>) {
        let database = test_db().await;
        seed::meeting(&database, MEETING, "Standup", at("2026-10-11T09:00:00Z")).await;
        let segments = SqliteSegmentRepo::new(database.clone());
        let rows: Vec<NewSegment> = (0_u32..)
            .zip(texts)
            .map(|(i, t)| segment(i * 1_000, t))
            .collect();
        segments.append(&rows).await.unwrap();
        let fixture = Self {
            corrections: SqliteCorrectionRepo::new(database),
            segments,
            meeting: MeetingId::new(MEETING),
        };
        (fixture, rows)
    }

    async fn texts(&self) -> Vec<String> {
        let segments = self.segments.list(&self.meeting, Page::ALL).await.unwrap();
        segments.into_iter().map(|segment| segment.text).collect()
    }

    /// Applies `corrections` at 10:00.
    async fn apply(&self, corrections: &[Correction]) -> u32 {
        let at = at("2026-10-11T10:00:00Z");
        self.corrections.apply(corrections, at).await.unwrap()
    }

    /// Undoes `scope` at 11:00.
    async fn undo(&self, scope: UndoScope<'_>) -> UndoReport {
        let at = at("2026-10-11T11:00:00Z");
        self.corrections.undo(scope, at).await.unwrap()
    }

    /// The meeting's corrections that are not undone.
    async fn listed(&self) -> Vec<CorrectedSegment> {
        self.corrections.corrections(&self.meeting).await.unwrap()
    }
}

#[tokio::test]
async fn applying_corrects_segments_and_undo_restores_the_engines_words() {
    let (fixture, rows) = Fixture::new(&["Sreyas", "hello"]).await;

    assert_eq!(fixture.apply(&[correction(&rows[0], "Shreyas")]).await, 1);
    assert_eq!(fixture.texts().await, ["Shreyas", "hello"]);
    let report = fixture.undo(UndoScope::Meeting(&fixture.meeting)).await;

    assert_eq!(
        report,
        UndoReport {
            restored: 1,
            kept: 0
        }
    );
    let restored = fixture
        .segments
        .list(&fixture.meeting, Page::ALL)
        .await
        .unwrap();
    assert_eq!(restored[0].text, "Sreyas");
    assert_eq!(restored[0].words, rows[0].words);
    assert!(fixture.listed().await.is_empty());
}

#[tokio::test]
async fn an_undone_correction_is_never_applied_again() {
    let (fixture, rows) = Fixture::new(&["Sreyas"]).await;
    let fix = correction(&rows[0], "Shreyas");
    fixture.apply(std::slice::from_ref(&fix)).await;
    fixture.undo(UndoScope::Segment(&rows[0].id)).await;

    let again = fixture.apply(&[fix]).await;

    assert_eq!(again, 0);
    assert_eq!(fixture.texts().await, ["Sreyas"]);
}

#[tokio::test]
async fn a_segment_edited_after_correction_is_neither_corrected_nor_restored() {
    let (fixture, rows) = Fixture::new(&["Sreyas", "Sreyas too"]).await;
    fixture.apply(&[correction(&rows[0], "Shreyas")]).await;
    for row in &rows {
        let edited = at("2026-10-11T10:30:00Z");
        fixture
            .segments
            .edit_text(&row.id, "edited by hand", edited)
            .await
            .unwrap();
    }

    let late = fixture.apply(&[correction(&rows[1], "Shreyas too")]).await;
    let listed = fixture.listed().await;
    let report = fixture.undo(UndoScope::Meeting(&fixture.meeting)).await;

    assert_eq!(late, 0);
    assert!(!listed[0].undoable);
    assert_eq!(
        report,
        UndoReport {
            restored: 0,
            kept: 1
        }
    );
    assert_eq!(fixture.texts().await, ["edited by hand", "edited by hand"]);
}

#[tokio::test]
async fn a_second_correction_keeps_the_engines_original() {
    let (fixture, rows) = Fixture::new(&["Sreyas"]).await;
    let live = correction(&rows[0], "Shreyas");
    fixture
        .segments
        .save_live(
            &[NewSegment {
                text: "Shreyas".into(),
                ..rows[0].clone()
            }],
            &[],
        )
        .await
        .unwrap();
    fixture
        .corrections
        .record(&[live], &[], at("2026-10-11T10:00:00Z"))
        .await
        .unwrap();

    let stage = Correction {
        original_text: "Shreyas".into(),
        ..correction(&rows[0], "Shreyas Patel")
    };
    fixture
        .corrections
        .apply(&[stage], at("2026-10-11T11:00:00Z"))
        .await
        .unwrap();

    let listed = fixture
        .corrections
        .corrections(&fixture.meeting)
        .await
        .unwrap();
    assert_eq!(listed[0].original_text, "Sreyas");
    assert_eq!(listed[0].corrected_text, "Shreyas Patel");
    assert!(listed[0].undoable);
}

#[tokio::test]
async fn recording_forgets_drafts_that_are_no_longer_corrected() {
    let (fixture, rows) = Fixture::new(&["Sreyas"]).await;
    let draft = correction(&rows[0], "Shreyas");
    fixture
        .corrections
        .record(&[draft], &[], at("2026-10-11T10:00:00Z"))
        .await
        .unwrap();

    fixture
        .corrections
        .record(&[], &[rows[0].id], at("2026-10-11T10:00:01Z"))
        .await
        .unwrap();

    assert!(
        fixture
            .corrections
            .corrections(&fixture.meeting)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn a_meetings_corrections_are_found_through_the_segment_index() {
    let database = test_db().await;

    let plan = query_plan(&database, include_str!("queries/kept.sql")).await;

    assert!(
        plan.iter()
            .any(|step| step.contains("idx_transcript_segments_meeting_id_start_ms")),
        "{plan:?}"
    );
}
