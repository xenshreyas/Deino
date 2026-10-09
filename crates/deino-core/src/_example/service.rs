//! The example's business logic.
//!
//! A service receives every dependency as a port (dependency inversion), so
//! tests swap in fakes and the composition root wires real adapters. It has
//! no SQL, no Tauri types and no wall-clock reads.

use std::sync::Arc;

use deino_domain::{Clock, ExampleNoteId};

use super::{ExampleError, ExampleEvents, ExampleNote, ExampleNoteRepo, NoteText};

/// Lists and adds notes.
pub struct ExampleService {
    repo: Arc<dyn ExampleNoteRepo>,
    events: Arc<dyn ExampleEvents>,
    clock: Arc<dyn Clock>,
}

impl ExampleService {
    /// Creates the service over its ports.
    pub fn new(
        repo: Arc<dyn ExampleNoteRepo>,
        events: Arc<dyn ExampleEvents>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            events,
            clock,
        }
    }

    /// Every note, newest first.
    pub async fn list(&self) -> Result<Vec<ExampleNote>, ExampleError> {
        Ok(self.repo.list().await?)
    }

    /// Validates `text`, saves it as a new note and announces it.
    pub async fn add(&self, text: &str) -> Result<ExampleNote, ExampleError> {
        let text = NoteText::parse(text)?;
        let note = ExampleNote {
            id: ExampleNoteId::generate(self.clock.as_ref()),
            text: text.into_inner(),
            created_at: self.clock.now(),
        };
        self.repo.insert(&note).await?;
        self.events.note_added(&note);
        // Log IDs and counts, never what the user wrote.
        log::debug!(target: "example", "added note {}", note.id);
        Ok(note)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use deino_domain::FakeClock;
    use deino_testkit::at;

    use super::*;
    use crate::_example::{FakeExampleNoteRepo, RecordedExampleEvents};

    struct Harness {
        service: ExampleService,
        events: Arc<RecordedExampleEvents>,
        clock: Arc<FakeClock>,
    }

    fn harness() -> Harness {
        let events = Arc::new(RecordedExampleEvents::default());
        let clock = Arc::new(FakeClock::new(at("2026-10-08T09:00:00Z")));
        let repo = Arc::new(FakeExampleNoteRepo::default());
        let service = ExampleService::new(repo, events.clone(), clock.clone());
        Harness {
            service,
            events,
            clock,
        }
    }

    #[tokio::test]
    async fn add_saves_the_trimmed_note_at_the_clock_time() {
        let Harness { service, clock, .. } = harness();

        let note = service.add("  buy milk ").await.unwrap();

        assert_eq!(note.text, "buy milk");
        assert_eq!(note.created_at, clock.now());
        assert_eq!(service.list().await.unwrap(), [note]);
    }

    #[tokio::test]
    async fn add_announces_the_saved_note() {
        let Harness {
            service, events, ..
        } = harness();

        let note = service.add("buy milk").await.unwrap();

        assert_eq!(events.added(), [note]);
    }

    #[tokio::test]
    async fn rejected_text_is_neither_saved_nor_announced() {
        let Harness {
            service, events, ..
        } = harness();

        let error = service.add(" ").await.unwrap_err();

        assert!(matches!(error, ExampleError::EmptyText));
        assert!(service.list().await.unwrap().is_empty());
        assert!(events.added().is_empty());
    }

    #[tokio::test]
    async fn list_returns_newest_first() {
        let Harness { service, clock, .. } = harness();
        let first = service.add("first").await.unwrap();
        clock.advance(TimeDelta::seconds(1));

        let second = service.add("second").await.unwrap();

        assert_eq!(service.list().await.unwrap(), [second, first]);
    }
}
