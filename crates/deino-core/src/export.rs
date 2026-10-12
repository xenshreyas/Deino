//! Exports (EXP-01, EXP-02, ADR 0025): meetings out of Deino as files, on
//! the clipboard, as email drafts, into a notes vault after every meeting,
//! and as one archive of the whole account.
//!
//! - [`ExportAssembler`] reads a meeting from the repositories into the
//!   `deino_export::MeetingExport` every renderer takes, with names resolved
//!   and times in the user's [`LocalZone`]. Notes come through the
//!   [`NoteRenderer`] port ([`DocNoteRenderer`]: the notepad through
//!   `deino-doc`, the summary as stored).
//! - [`ExportService`]: preview, save (through the [`SaveDialog`] port: the
//!   webview never names a path), copy as rich text or Markdown (the
//!   [`Clipboard`] port) and email (the [`UrlOpener`] port); `save_many`
//!   writes many meetings into one folder the user picks (ORG-07).
//! - [`AutoExportService`] is the [`AutoExporter`] behind the pipeline's
//!   `auto_export` stage: notes into the chosen folder by the settings'
//!   templates, Obsidian links and people pages, overwriting only its own
//!   files (recognised by their `deino-id`).
//! - [`FullExportService`]: the account export, a background job with
//!   progress ([`FullExportEvents`]) writing a zip of every meeting.
//! - [`Vault`] ([`FsVault`]) is the file system: atomic writes, and the
//!   head of a file to tell whose it is.
//!
//! With Settings → Privacy → "Hide sensitive information" on, the assembler
//! redacts secrets (NOT-09, [`ExportPorts::with_redactor`]).
//!
//! Rendering itself is `deino-export`; the dialogs, clipboard and opener
//! are app adapters.

mod archive;
mod assemble;
mod auto;
mod batch;
mod error;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod full;
mod model;
mod notes;
mod platform;
mod redacted;
mod service;
#[cfg(test)]
mod testing;
mod vault;

pub use assemble::{ExportAssembler, ExportPorts};
pub use auto::{AutoExportOutcome, AutoExportService, AutoExportSkip, AutoExporter, PEOPLE_FOLDER};
/// The time-zone port is shared with action items (ADR 0028).
pub use deino_domain::{FixedZone, LocalZone, SystemZone};
pub use error::ExportError;
#[cfg(any(test, feature = "test-support"))]
pub use fake::{FakeVault, RecordedClipboard, RecordedOpener, ScriptedDialog};
pub use full::{FullExportDeps, FullExportEvents, FullExportService};
pub use model::{
    CopyKind, ExportPreview, FullExportState, FullExportStatus, PreviewKind, SavedExport,
    SavedExports,
};
pub use notes::{DocNoteRenderer, MeetingNotes, NoteRenderer};
pub use platform::{Clipboard, ClipboardContent, FolderDialog, SaveDialog, SaveRequest, UrlOpener};
pub use redacted::RedactedNotes;
pub use service::{ExportService, ExportServiceDeps};
pub use vault::{FsVault, HEAD_BYTES, Vault};
