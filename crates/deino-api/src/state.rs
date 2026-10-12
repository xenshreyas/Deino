//! [`ApiState`]: the services every handler reaches, as the composition
//! root (or a test) wires them.

use std::sync::Arc;

use deino_core::action_items::ActionItemService;
use deino_core::api_keys::ApiKeyService;
use deino_core::lock::LockService;
use deino_core::meetings::{MeetingService, TrashService};
use deino_core::notes::NoteService;
use deino_core::query::Library;
use deino_core::redaction::TextRedactor;
use deino_core::tags::TagService;
use deino_core::transcript_import::TranscriptImporter;
use deino_domain::LocalZone;

/// Whether Deino is locked (PRV-02). While it is, the API refuses every
/// request with `423 Locked` (ADR 2026-10-12-local-rest-api).
pub trait LockProbe: Send + Sync {
    /// True while the app is locked.
    fn is_locked(&self) -> bool;
}

impl LockProbe for LockService {
    fn is_locked(&self) -> bool {
        LockService::is_locked(self)
    }
}

/// Never locked: for builds and tests without an app lock.
#[derive(Clone, Copy, Debug, Default)]
pub struct NeverLocked;

impl LockProbe for NeverLocked {
    fn is_locked(&self) -> bool {
        false
    }
}

/// The services writes go through: the app's own, so every change takes
/// the single writer and the UI hears of it through `DbChanged`.
#[derive(Clone)]
pub struct WriteServices {
    /// Renames meetings.
    pub meetings: Arc<MeetingService>,
    /// Moves meetings to the trash (ORG-07).
    pub trash: Arc<TrashService>,
    /// Saves "my notes".
    pub notes: Arc<NoteService>,
    /// Makes, edits, applies and deletes tags.
    pub tags: Arc<TagService>,
    /// Makes, edits and deletes action items.
    pub action_items: Arc<ActionItemService>,
    /// Imports a transcript as a meeting.
    pub importer: Arc<TranscriptImporter>,
}

/// Everything the API reads and writes through.
#[derive(Clone)]
pub struct ApiServices {
    /// Every read: the same layer as the `deino` CLI (ADR 0035), so the
    /// API returns what `deino --json` and the MCP tools return.
    pub library: Arc<Library>,
    /// Hides secrets in search snippets when the user asks (NOT-09); the
    /// library already redacts notes, transcripts and action items.
    pub redactor: Arc<dyn TextRedactor>,
    /// The user's time zone, for action items' due days.
    pub zone: Arc<dyn LocalZone>,
    /// Checks keys.
    pub keys: Arc<ApiKeyService>,
    /// The app lock.
    pub lock: Arc<dyn LockProbe>,
    /// The writes.
    pub writes: WriteServices,
}

/// What handlers share: the services and the port the server listens on,
/// for the `Host` check.
#[derive(Clone)]
pub struct ApiState {
    /// The services.
    pub services: Arc<ApiServices>,
    /// The port the server is bound to; `None` in tests that never bind.
    pub port: Option<u16>,
}
