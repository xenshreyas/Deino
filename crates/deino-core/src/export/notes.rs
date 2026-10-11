//! The meeting's notes as Markdown, behind the [`NoteRenderer`] port.
//!
//! The user's notes are the notepad's TipTap document (NOT-01) and the AI
//! notes the newest version of the enhanced notes (NOT-02, read through the
//! stable [`EnhancedNotesReader`]); `deino-doc` writes both out as Readable
//! Markdown (mentions as `@Name`, citations as `[12:04]`).

use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_doc::{Doc, Flavor, to_markdown};
use deino_domain::MeetingId;

use crate::enhance::EnhancedNotesReader;
use crate::notes::NotesRepo;

/// A meeting's notes as Markdown; `None` when there are none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeetingNotes {
    /// The AI notes: the newest enhanced notes.
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

/// [`NoteRenderer`] over the enhanced notes and the notepad's documents.
#[derive(Clone)]
pub struct DocNoteRenderer {
    enhanced: Arc<dyn EnhancedNotesReader>,
    notepad: Arc<dyn NotesRepo>,
}

impl DocNoteRenderer {
    /// Reads the AI notes through `enhanced` and the user's through `notepad`.
    pub fn new(enhanced: Arc<dyn EnhancedNotesReader>, notepad: Arc<dyn NotesRepo>) -> Self {
        Self { enhanced, notepad }
    }
}

#[async_trait]
impl NoteRenderer for DocNoteRenderer {
    async fn notes(&self, meeting: &MeetingId) -> Result<MeetingNotes, DbError> {
        let enhanced = self.enhanced.latest(meeting).await?;
        let mine = self.notepad.get(meeting).await?;
        Ok(MeetingNotes {
            enhanced: enhanced.and_then(|notes| readable(&notes.doc)),
            mine: mine.and_then(|notes| readable(&notes.doc)),
        })
    }
}

/// `doc` as Readable Markdown; `None` when it says nothing.
fn readable(doc: &Doc) -> Option<String> {
    let markdown = to_markdown(doc, Flavor::Readable);
    (!markdown.trim().is_empty()).then_some(markdown)
}

#[cfg(test)]
mod tests {
    use deino_doc::from_markdown;
    use deino_testkit::{at, seed, test_db};

    use super::*;
    use crate::enhance::{EnhancedNotesWriter, NewVersion, SqliteEnhancedNotesRepo};
    use crate::notes::{SqliteNotesRepo, StoredNotes as Notepad};

    async fn notes_of(enhanced: Option<&str>, mine: Option<&str>) -> MeetingNotes {
        let database = test_db().await;
        let meeting = MeetingId::new("meeting-1");
        seed::meeting(
            &database,
            "meeting-1",
            "Standup",
            at("2026-10-08T09:00:00Z"),
        )
        .await;
        let repo = Arc::new(SqliteEnhancedNotesRepo::new(database.clone()));
        if let Some(markdown) = enhanced {
            let version = NewVersion {
                meeting_id: meeting.clone(),
                doc: from_markdown(markdown).unwrap(),
                markdown: markdown.into(),
                template_id: None,
                model: "builtin:test".into(),
                basis: None,
                is_user_edited: false,
                at: at("2026-10-08T10:00:00Z"),
            };
            repo.insert(&version).await.unwrap();
        }
        let notepad = Arc::new(SqliteNotesRepo::new(database.clone()));
        if let Some(markdown) = mine {
            let stored = Notepad {
                meeting_id: meeting.clone(),
                doc: from_markdown(markdown).unwrap(),
                markdown: String::new(),
                created_at: at("2026-10-08T09:00:00Z"),
                updated_at: at("2026-10-08T09:00:00Z"),
            };
            notepad.save(&stored).await.unwrap();
        }
        DocNoteRenderer::new(repo, notepad)
            .notes(&meeting)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn both_notes_are_read_as_readable_markdown() {
        let notes = notes_of(
            Some("### Overview\n\n- shipped"),
            Some("- [ ] **Ship** the fix"),
        )
        .await;

        assert_eq!(
            notes.enhanced.as_deref(),
            Some("### Overview\n\n- shipped\n")
        );
        assert_eq!(notes.mine.as_deref(), Some("- [ ] **Ship** the fix\n"));
    }

    #[tokio::test]
    async fn a_meeting_without_notes_or_with_blank_ones_has_none() {
        assert_eq!(notes_of(None, None).await, MeetingNotes::default());
        assert_eq!(notes_of(None, Some("  ")).await, MeetingNotes::default());
    }
}
