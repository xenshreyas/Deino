//! Screen capture's stored state (CAP-09): the snapshots kept while a
//! meeting records, the text read in them, and the per-meeting off switch.
//!
//! `deino-screen` captures; the app's snapshot store appends each kept
//! picture through [`ScreenRepo::add_snapshot`], and "Turn off for this
//! meeting" writes [`ScreenRepo::set_capture`], which the next start (a
//! resumed recording) reads with [`ScreenRepo::capture_choice`]. The
//! meeting page's Images strip reads [`ScreenRepo::snapshots`] and deletes
//! through [`ScreenService::delete_snapshot`].
//!
//! After the recording, the `screen_context` stage runs
//! [`ScreenContextService`]: Apple Vision reads each snapshot's text
//! ([`ScreenOcr`]) and, when the user allows it, Claude describes it
//! ([`GatewayDescriber`], route `vision.describe`). NOT-02's notes read the
//! result through [`ScreenTexts`], time-aligned with the transcript.
//!
//! | File | Role |
//! |---|---|
//! | `model.rs` | [`NewSnapshot`], [`Snapshot`], [`ScreenText`] |
//! | `repo.rs`, `text_repo.rs` | [`ScreenRepo`], [`SnapshotTextRepo`], [`ScreenTexts`] over SQLite |
//! | `service.rs` | [`ScreenService`]: the off switch, the list, deletion |
//! | `context.rs` | [`ScreenContextService`]: OCR and descriptions, the stage's rules |
//! | `describe.rs` | [`GatewayDescriber`]: `vision.describe` over the gateway |
//! | `consent.rs` | [`ConsentReminders`]: the first three captures' reminder |

#[cfg(test)]
mod acceptance_tests;
mod consent;
mod context;
mod describe;
#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod repo;
mod service;
mod text_repo;

pub use consent::{ConsentReminders, ReminderCounter};
pub use context::{
    DescribeError, MAX_DESCRIBED, MAX_TEXT_CHARS, NoDescriptions, OcrError, SNAPSHOTS_DIR,
    ScreenContextError, ScreenContextParts, ScreenContextReport, ScreenContextService, ScreenOcr,
    SnapshotDescriber, clean,
};
pub use describe::{DescribeSetting, GatewayDescriber, describe_prompt};
#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeScreenRepo;
pub use model::{NewSnapshot, ScreenText, Snapshot};
pub use repo::{ScreenRepo, SqliteScreenRepo};
pub use service::{CaptureControl, ScreenService};
pub use text_repo::{NoScreenTexts, ScreenTexts, SnapshotTextRepo};
