//! In-memory notes ports for tests, honouring the traits' contracts.

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::MeetingId;

use super::{ImageName, NoteImages, NotesEvents, NotesRepo, SavedNotes, StoredNotes};
use crate::testing::lock;

/// Keeps notes in a map.
#[derive(Debug, Default)]
pub struct FakeNotesRepo {
    notes: Mutex<BTreeMap<MeetingId, StoredNotes>>,
}

#[async_trait]
impl NotesRepo for FakeNotesRepo {
    async fn get(&self, meeting: &MeetingId) -> Result<Option<StoredNotes>, DbError> {
        Ok(lock(&self.notes).get(meeting).cloned())
    }

    async fn save(&self, notes: &StoredNotes) -> Result<(), DbError> {
        let mut stored = lock(&self.notes);
        let created_at = stored
            .get(&notes.meeting_id)
            .map_or(notes.created_at, |existing| existing.created_at);
        let mut notes = notes.clone();
        notes.created_at = created_at;
        stored.insert(notes.meeting_id.clone(), notes);
        Ok(())
    }
}

/// Keeps images in memory, under a made-up `/meetings` root.
#[derive(Debug, Default)]
pub struct MemoryNoteImages {
    files: Mutex<BTreeMap<(MeetingId, String), Vec<u8>>>,
}

impl MemoryNoteImages {
    /// The names of the meeting's images.
    pub fn names(&self, meeting: &MeetingId) -> Vec<String> {
        lock(&self.files)
            .keys()
            .filter(|(owner, _)| owner == meeting)
            .map(|(_, name)| name.clone())
            .collect()
    }
}

#[async_trait]
impl NoteImages for MemoryNoteImages {
    fn folder(&self, meeting: &MeetingId) -> io::Result<PathBuf> {
        Ok(PathBuf::from("/meetings").join(meeting.as_str()))
    }

    async fn write(&self, meeting: &MeetingId, name: &ImageName, bytes: &[u8]) -> io::Result<()> {
        let key = (meeting.clone(), name.to_string());
        lock(&self.files).insert(key, bytes.to_vec());
        Ok(())
    }
}

/// Records every save announced.
#[derive(Debug, Default)]
pub struct RecordedNotesEvents {
    saved: Mutex<Vec<SavedNotes>>,
}

impl RecordedNotesEvents {
    /// The saves announced, in order.
    pub fn announced(&self) -> Vec<SavedNotes> {
        lock(&self.saved).clone()
    }
}

impl NotesEvents for RecordedNotesEvents {
    fn saved(&self, saved: &SavedNotes) {
        lock(&self.saved).push(saved.clone());
    }
}
