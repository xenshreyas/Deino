//! Transcripts imported as meetings (DEV-03's `POST /v1/meetings`).
//!
//! [`TranscriptImporter`] takes a [`TranscriptImport`] in Circleback's
//! import shape (C §12.1) and writes it through the meeting, speaker,
//! segment, enhanced-notes and tag services and repositories, like the
//! sample meeting (FND-11): a `ready` meeting from the `import` source, one
//! speaker per name ("Me" is the user), the notes as AI notes version 1,
//! and tags matched by name or created. With [`ImportPeoplePorts`] (CAP-12's
//! importers) attendees become people on the meeting and action items are
//! added. The pipeline then indexes it for search. A failed import removes
//! what it wrote.

mod error;
mod model;
mod people;
mod service;

pub use error::TranscriptImportError;
pub use model::{
    ImportActionItem, ImportAttendee, ImportTag, ImportUtterance, ImportWord, MAX_UTTERANCES,
    TimedUtterance, TranscriptImport, WordUtterance,
};
pub use people::ImportPeoplePorts;
pub use service::{TranscriptImportPorts, TranscriptImporter};
