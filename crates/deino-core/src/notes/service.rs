//! The notes rules: what may be saved, where images go, and how a meeting
//! is made for notes written before it.
//!
//! Every save is a whole document (the editor autosaves about once a
//! second), normalized and validated by `deino-doc`, with its Markdown
//! derived here so readers never parse JSON. Saving continuously is what
//! makes notes typed during a recording survive a crash.

use std::sync::Arc;

use deino_doc::{Doc, Flavor};
use deino_domain::{Clock, MeetingId, MeetingSource, MeetingStatus};
use uuid::{NoContext, Timestamp, Uuid};

use super::{
    ImageKind, ImageName, MAX_DOC_BYTES, MAX_IMAGE_BYTES, MeetingNotes, NoteImage, NoteImages,
    NotesError, NotesEvents, NotesRepo, SavedNotes, StoredNotes,
};
use crate::meetings::{Meeting, MeetingLifecycleRepo, MeetingState, MeetingTitle, NewMeeting};

/// The ports the service works through, wired by the composition root.
pub struct NotePorts {
    /// Where notes are stored.
    pub notes: Arc<dyn NotesRepo>,
    /// Meetings, to check they exist and to create scheduled ones.
    pub meetings: Arc<dyn MeetingLifecycleRepo>,
    /// The meetings' image folders.
    pub images: Arc<dyn NoteImages>,
    /// Who hears about saves.
    pub events: Arc<dyn NotesEvents>,
}

/// Opens and saves meeting notes and their images.
pub struct NoteService {
    pub(super) ports: NotePorts,
    pub(super) clock: Arc<dyn Clock>,
}

impl NoteService {
    /// Creates the service over its ports.
    pub fn new(ports: NotePorts, clock: Arc<dyn Clock>) -> Self {
        Self { ports, clock }
    }

    /// The meeting's notes, or an empty document if none were written.
    pub async fn get(&self, meeting: &MeetingId) -> Result<MeetingNotes, NotesError> {
        let stored = self.ports.notes.get(meeting).await?;
        if stored.is_none() {
            self.meeting(meeting).await?;
        }
        Ok(MeetingNotes {
            meeting_id: meeting.clone(),
            folder: self.folder(meeting)?,
            updated_at: stored.as_ref().map(|notes| notes.updated_at),
            doc: stored.map_or_else(Doc::empty, |notes| notes.doc),
        })
    }

    /// The meeting's notes as readable Markdown, for LLM context; `None`
    /// when nothing (or only whitespace) was written.
    pub async fn markdown(&self, meeting: &MeetingId) -> Result<Option<String>, NotesError> {
        let stored = self.ports.notes.get(meeting).await?;
        Ok(stored
            .map(|notes| notes.markdown)
            .filter(|markdown| !markdown.trim().is_empty()))
    }

    /// Saves `doc` as the meeting's notes, replacing what was there.
    pub async fn save(&self, meeting: &MeetingId, doc: Doc) -> Result<SavedNotes, NotesError> {
        self.meeting(meeting).await?;
        let notes = self.prepare(meeting, doc)?;
        self.ports.notes.save(&notes).await?;
        Ok(self.announce(&notes))
    }

    /// Creates a `scheduled` meeting to hold notes written ahead of a
    /// recording (the record screen's notepad, and later ORG-02's upcoming
    /// events). Fails with the meetings feature's title errors.
    pub async fn create_scheduled(&self, title: &str) -> Result<Meeting, NotesError> {
        let title = MeetingTitle::parse(title)?;
        let now = self.clock.now();
        let meeting = NewMeeting {
            id: MeetingId::new(format!("meeting-{}", self.new_uuid().hyphenated())),
            title,
            status: MeetingStatus::Scheduled,
            source: MeetingSource::Live,
            created_at: now,
            started_at: None,
            folder_path: None,
        };
        self.ports.meetings.create(&meeting).await?;
        log::info!(target: "notes", "created scheduled meeting {}", meeting.id);
        Ok(Meeting {
            id: meeting.id,
            title: meeting.title.as_str().to_owned(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Stores a pasted or dropped image in the meeting's folder. Only PNG,
    /// JPEG, GIF and WebP up to [`MAX_IMAGE_BYTES`] are accepted, judged by
    /// content; the file name is generated, never taken from the caller.
    pub async fn save_image(
        &self,
        meeting: &MeetingId,
        bytes: &[u8],
    ) -> Result<NoteImage, NotesError> {
        self.meeting(meeting).await?;
        let kind = ImageKind::sniff(bytes).ok_or(NotesError::UnsupportedImage)?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(NotesError::ImageTooLarge);
        }
        let name = ImageName::new(self.new_uuid(), kind);
        self.ports
            .images
            .write(meeting, &name, bytes)
            .await
            .map_err(NotesError::Files)?;
        log::debug!(target: "notes", "saved a {} byte image for {meeting}", bytes.len());
        Ok(NoteImage { src: name.src() })
    }

    /// The meeting's lifecycle state; [`NotesError::NotFound`] if it is gone.
    pub(super) async fn meeting(&self, meeting: &MeetingId) -> Result<MeetingState, NotesError> {
        self.ports
            .meetings
            .state(meeting)
            .await?
            .ok_or_else(|| NotesError::NotFound(meeting.clone()))
    }

    /// Normalizes and checks `doc`, and derives its Markdown.
    pub(super) fn prepare(&self, meeting: &MeetingId, doc: Doc) -> Result<StoredNotes, NotesError> {
        let doc = doc.normalized();
        deino_doc::validate(&doc).map_err(NotesError::InvalidDoc)?;
        if !doc
            .images()
            .iter()
            .all(|image| ImageName::from_src(&image.src).is_some())
        {
            return Err(NotesError::ForeignImage);
        }
        if serde_json::to_vec(&doc).map_or(usize::MAX, |json| json.len()) > MAX_DOC_BYTES {
            return Err(NotesError::TooLarge);
        }
        let now = self.clock.now();
        Ok(StoredNotes {
            meeting_id: meeting.clone(),
            markdown: deino_doc::to_markdown(&doc, Flavor::Readable),
            doc,
            created_at: now,
            updated_at: now,
        })
    }

    /// Tells listeners the notes were saved, and logs it without content.
    pub(super) fn announce(&self, notes: &StoredNotes) -> SavedNotes {
        let saved = SavedNotes {
            meeting_id: notes.meeting_id.clone(),
            updated_at: notes.updated_at,
        };
        self.ports.events.saved(&saved);
        log::debug!(
            target: "notes",
            "saved notes for {} ({} blocks)",
            saved.meeting_id,
            notes.doc.content.len()
        );
        saved
    }

    fn folder(&self, meeting: &MeetingId) -> Result<String, NotesError> {
        let folder = self
            .ports
            .images
            .folder(meeting)
            .map_err(NotesError::Files)?;
        Ok(folder.to_string_lossy().into_owned())
    }

    /// A UUIDv7 stamped by the injected clock.
    fn new_uuid(&self) -> Uuid {
        let now = self.clock.now();
        let seconds = u64::try_from(now.timestamp()).unwrap_or_default();
        Uuid::new_v7(Timestamp::from_unix(
            NoContext,
            seconds,
            now.timestamp_subsec_nanos(),
        ))
    }
}
