//! The meeting's notes as Markdown, behind the [`NoteRenderer`] port.
//!
//! Today both notes are stored as Markdown: the AI notes in the legacy
//! summary's result (`summary_processes.result`, `{ "markdown": … }`) and the
//! user's notes in `meeting_notes.notes_markdown`. [`StoredMarkdownNotes`]
//! reads them as they are. When NOT-01 stores the notepad as TipTap JSON and
//! NOT-02 the enhanced notes as blocks, a renderer over `deino-doc` takes its
//! place and no export code changes.

use async_trait::async_trait;
use deino_db::{Database, DbError};
use deino_domain::MeetingId;
use serde::Deserialize;

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

/// [`NoteRenderer`] over the Markdown the legacy summary and notepad store.
#[derive(Clone, Debug)]
pub struct StoredMarkdownNotes {
    database: Database,
}

impl StoredMarkdownNotes {
    /// Reads notes from `database`.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

/// The part of the legacy summary result that holds Markdown. Results in
/// the pre-Markdown section format have none and export without AI notes.
#[derive(Deserialize)]
struct StoredSummary {
    markdown: Option<String>,
}

#[async_trait]
impl NoteRenderer for StoredMarkdownNotes {
    async fn notes(&self, meeting: &MeetingId) -> Result<MeetingNotes, DbError> {
        let summary = sqlx::query_scalar!(
            "SELECT result FROM summary_processes WHERE meeting_id = ?1",
            meeting,
        )
        .fetch_optional(self.database.reader())
        .await?
        .flatten();
        let mine = sqlx::query_scalar!(
            "SELECT notes_markdown FROM meeting_notes WHERE meeting_id = ?1",
            meeting,
        )
        .fetch_optional(self.database.reader())
        .await?
        .flatten();
        Ok(MeetingNotes {
            enhanced: summary.as_deref().and_then(summary_markdown),
            mine: mine.filter(|text| !text.trim().is_empty()),
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
                 VALUES ('meeting-1', ?1, '', '')",
            )
            .bind(text)
            .execute(database.writer())
            .await
            .unwrap();
        }
        StoredMarkdownNotes::new(database)
            .notes(&MeetingId::new("meeting-1"))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn both_notes_are_read_as_markdown() {
        let summary = serde_json::json!({ "markdown": "## Overview\n- shipped" }).to_string();
        let notes = notes_of(Some(&summary), Some("- my point")).await;

        assert_eq!(notes.enhanced.as_deref(), Some("## Overview\n- shipped"));
        assert_eq!(notes.mine.as_deref(), Some("- my point"));
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
