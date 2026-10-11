//! The meeting's notes as Markdown, behind the [`NoteRenderer`] port.
//!
//! The user's notes are the notepad's TipTap document (NOT-01), read through
//! the [`NotesRepo`] port and written out by `deino-doc` as Readable
//! Markdown (mentions as `@Name`, citations as `[12:04]`). The AI notes are
//! still the legacy summary's Markdown (`summary_processes.result`,
//! `{ "markdown": … }`); when NOT-02 stores enhanced notes as a document,
//! [`DocNoteRenderer`] renders them through `deino-doc` the same way and no
//! export code changes.

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::{Database, DbError};
use deino_doc::{Flavor, to_markdown};
use deino_domain::MeetingId;
use serde::Deserialize;

use crate::notes::NotesRepo;

/// A meeting's notes as Markdown; `None` when there are none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingNotes {
    /// The AI notes (summary today, enhanced notes after NOT-02).
    pub enhanced: Option<String>,
    /// What the user typed.
    pub mine: Option<String>,
}

/// Renders a meeting's stored notes as Markdown.
#[async_trait]
pub trait NoteRenderer: Send + Sync {
    /// The meeting's notes; empty for a meeting without notes.
    async fn notes(&self, meeting: &MeetingId) -> Result<MeetingNotes, DbError>;
}

/// [`NoteRenderer`] over the notepad's documents and the legacy summary.
#[derive(Clone)]
pub struct DocNoteRenderer {
    database: Database,
    notepad: Arc<dyn NotesRepo>,
}

impl DocNoteRenderer {
    /// Reads summaries from `database` and the notepad through `notepad`.
    pub fn new(database: Database, notepad: Arc<dyn NotesRepo>) -> Self {
        Self { database, notepad }
    }

    async fn mine(&self, meeting: &MeetingId) -> Result<Option<String>, DbError> {
        let stored = self.notepad.get(meeting).await?;
        let markdown = stored.map(|notes| to_markdown(&notes.doc, Flavor::Readable));
        Ok(markdown.filter(|text| !text.trim().is_empty()))
    }
}

/// The part of the legacy summary result that holds Markdown. Results in
/// the pre-Markdown section format have none and export without AI notes.
#[derive(Deserialize)]
struct StoredSummary {
    markdown: Option<String>,
}

#[async_trait]
impl NoteRenderer for DocNoteRenderer {
    async fn notes(&self, meeting: &MeetingId) -> Result<MeetingNotes, DbError> {
        let summary = sqlx::query_scalar!(
            "SELECT result FROM summary_processes WHERE meeting_id = ?1",
            meeting,
        )
        .fetch_optional(self.database.reader())
        .await?
        .flatten();
        Ok(MeetingNotes {
            enhanced: summary.as_deref().and_then(summary_markdown),
            mine: self.mine(meeting).await?,
        })
    }
}

/// The Markdown of a stored summary result, if it has any.
fn summary_markdown(result: &str) -> Option<String> {
    serde_json::from_str::<StoredSummary>(result)
        .ok()?
        .markdown
        .filter(|markdown| !markdown.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use deino_testkit::{at, seed, test_db};

    use super::*;
    use crate::notes::{SqliteNotesRepo, StoredNotes as Notepad};

    async fn notes_of(summary: Option<&str>, mine: Option<&str>) -> MeetingNotes {
        let database = test_db().await;
        seed::meeting(
            &database,
            "meeting-1",
            "Standup",
            at("2026-10-08T09:00:00Z"),
        )
        .await;
        if let Some(result) = summary {
            sqlx::query(
                "INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, result)
                 VALUES ('meeting-1', 'completed', '', '', ?1)",
            )
            .bind(result)
            .execute(database.writer())
            .await
            .unwrap();
        }
        if let Some(text) = mine {
            sqlx::query(
                "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at)
                 VALUES ('meeting-1', ?1, '2026-10-08T09:00:00Z', '2026-10-08T09:00:00Z')",
            )
            .bind(text)
            .execute(database.writer())
            .await
            .unwrap();
        }
        let notepad = Arc::new(SqliteNotesRepo::new(database.clone()));
        DocNoteRenderer::new(database, notepad)
            .notes(&MeetingId::new("meeting-1"))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn both_notes_are_read_as_markdown() {
        let summary = serde_json::json!({ "markdown": "## Overview\n- shipped" }).to_string();
        let notes = notes_of(Some(&summary), Some("- my point")).await;

        assert_eq!(notes.enhanced.as_deref(), Some("## Overview\n- shipped"));
        assert_eq!(notes.mine.as_deref(), Some("- my point\n"));
    }

    #[tokio::test]
    async fn the_notepad_document_is_written_out_as_readable_markdown() {
        let database = test_db().await;
        seed::meeting(
            &database,
            "meeting-1",
            "Standup",
            at("2026-10-08T09:00:00Z"),
        )
        .await;
        let notepad = Arc::new(SqliteNotesRepo::new(database.clone()));
        let doc = deino_doc::from_markdown("- [ ] **Ship** the fix").unwrap();
        let stored = Notepad {
            meeting_id: MeetingId::new("meeting-1"),
            doc,
            markdown: String::new(),
            created_at: at("2026-10-08T09:00:00Z"),
            updated_at: at("2026-10-08T09:00:00Z"),
        };
        notepad.save(&stored).await.unwrap();

        let notes = DocNoteRenderer::new(database, notepad)
            .notes(&stored.meeting_id)
            .await
            .unwrap();

        assert_eq!(notes.mine.as_deref(), Some("- [ ] **Ship** the fix\n"));
    }

    #[tokio::test]
    async fn a_meeting_without_notes_has_none() {
        assert_eq!(notes_of(None, None).await, MeetingNotes::default());
    }

    #[tokio::test]
    async fn legacy_section_summaries_and_blank_notes_count_as_none() {
        let notes = notes_of(
            Some(r#"{"MeetingName":"x","_section_order":[]}"#),
            Some("  \n"),
        )
        .await;
        assert_eq!(notes, MeetingNotes::default());
    }
}
