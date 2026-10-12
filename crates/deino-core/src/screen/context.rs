//! [`ScreenContextService`]: the `screen_context` stage's rules (CAP-09).
//!
//! After a recording, each kept snapshot is read once with Apple Vision
//! ([`ScreenOcr`], run off the runtime) and its text stored on its row
//! (`""` when there is none, so a re-run reads only new snapshots). When
//! Settings → Screen capture → "Describe snapshots with Claude" is on and
//! Claude serves `vision.describe` ([`SnapshotDescriber::enabled`]), each
//! read snapshot is also described, the picture sent through the LLM
//! gateway. A snapshot whose text the NOT-09 redactor would hide is never
//! sent: its pixels cannot be redacted. A description that fails is left
//! for the next run and never fails the stage. NOT-02's notes then read
//! the text beside the transcript ([`ScreenTexts`](super::ScreenTexts)).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use deino_db::DbError;
use deino_domain::{ErrorClass, ErrorCode, MeetingId};

use super::{Snapshot, SnapshotTextRepo};
use crate::redaction::TextRedactor;

/// The snapshots' folder inside a meeting's recording folder (the same
/// name as `deino_screen::SNAPSHOTS`, which the app's tests compare).
pub const SNAPSHOTS_DIR: &str = "snapshots";
/// The most text kept of one snapshot, in characters.
pub const MAX_TEXT_CHARS: usize = 4_000;
/// The most snapshots of one meeting described (each is one LLM call).
pub const MAX_DESCRIBED: usize = 30;
/// The log target.
const TARGET: &str = "screen";

/// Reads the text in a picture (Apple Vision on macOS).
pub trait ScreenOcr: Send + Sync {
    /// The text in the PNG at `png`, one recognised line per line, `""`
    /// when there is none. Blocking: callers run it off the runtime.
    fn read(&self, png: &Path) -> Result<String, OcrError>;
}

/// Why a picture could not be read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OcrError {
    /// This platform has no text recognition.
    #[error("text recognition is not available on this platform")]
    Unsupported,
    /// The picture could not be read.
    #[error("could not read the snapshot: {0}")]
    Failed(String),
}

/// Describes snapshots with a multimodal model (`vision.describe`).
#[async_trait]
pub trait SnapshotDescriber: Send + Sync {
    /// Whether snapshots are described now: the setting is on and Claude
    /// serves the route.
    async fn enabled(&self) -> bool;

    /// A short description of the PNG `png`, sent to the model.
    async fn describe(&self, png: Vec<u8>) -> Result<String, DescribeError>;
}

/// Why a description failed (user-safe).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("could not describe the snapshot: {0}")]
pub struct DescribeError(pub String);

/// Snapshots are never described: tests, and builds without the gateway.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoDescriptions;

#[async_trait]
impl SnapshotDescriber for NoDescriptions {
    async fn enabled(&self) -> bool {
        false
    }

    async fn describe(&self, _png: Vec<u8>) -> Result<String, DescribeError> {
        Err(DescribeError("describing snapshots is off".to_owned()))
    }
}

/// What [`ScreenContextService`] is built over.
#[derive(Clone)]
pub struct ScreenContextParts {
    /// The snapshot rows.
    pub repo: Arc<dyn SnapshotTextRepo>,
    /// Apple Vision.
    pub ocr: Arc<dyn ScreenOcr>,
    /// `vision.describe`, when on.
    pub describer: Arc<dyn SnapshotDescriber>,
    /// NOT-09: a snapshot whose text it would hide is never sent.
    pub redactor: Arc<dyn TextRedactor>,
    /// The folder holding every meeting's recording folder.
    pub recordings: PathBuf,
}

/// What one run did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenContextReport {
    /// Snapshots read with OCR.
    pub read: usize,
    /// Snapshots described.
    pub described: usize,
}

/// Why the screen could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ScreenContextError {
    /// The rows could not be read or written.
    #[error(transparent)]
    Db(#[from] DbError),
    /// This platform has no text recognition.
    #[error("text recognition is not available on this platform")]
    Unsupported,
}

impl ErrorClass for ScreenContextError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Db(error) => error.code(),
            Self::Unsupported => ErrorCode::Unavailable,
        }
    }
}

/// The `screen_context` stage's rules (see the module docs).
pub struct ScreenContextService {
    parts: ScreenContextParts,
}

impl ScreenContextService {
    /// The service over its parts.
    pub fn new(parts: ScreenContextParts) -> Self {
        Self { parts }
    }

    /// Reads the snapshots of `meeting` not read yet, then describes the
    /// read ones when describing is on.
    pub async fn read(&self, meeting: &MeetingId) -> Result<ScreenContextReport, ScreenContextError> {
        let folder = self
            .parts
            .recordings
            .join(meeting.as_str())
            .join(SNAPSHOTS_DIR);
        let mut report = ScreenContextReport::default();
        for snapshot in self.parts.repo.unread(meeting).await? {
            let text = self.recognise(folder.join(&snapshot.file_name)).await?;
            self.parts.repo.set_text(snapshot.id, &text).await?;
            report.read += 1;
        }
        if self.parts.describer.enabled().await {
            report.described = self.describe_all(meeting, &folder).await?;
        }
        Ok(report)
    }

    /// The text of the PNG at `path`, trimmed and capped; `""` when the
    /// picture is gone or unreadable (logged), so it is not read again.
    async fn recognise(&self, path: PathBuf) -> Result<String, ScreenContextError> {
        let ocr = Arc::clone(&self.parts.ocr);
        let read = tokio::task::spawn_blocking(move || ocr.read(&path))
            .await
            .unwrap_or_else(|_| Err(OcrError::Failed("the reader stopped".to_owned())));
        match read {
            Ok(text) => Ok(clean(&text)),
            Err(OcrError::Unsupported) => Err(ScreenContextError::Unsupported),
            Err(error) => {
                log::warn!(target: TARGET, "a snapshot could not be read: {error}");
                Ok(String::new())
            }
        }
    }

    /// Describes the read, undescribed snapshots of `meeting`, at most
    /// [`MAX_DESCRIBED`] a run; the number described.
    async fn describe_all(&self, meeting: &MeetingId, folder: &Path) -> Result<usize, DbError> {
        let mut described = 0;
        for snapshot in self.parts.repo.undescribed(meeting).await? {
            if described == MAX_DESCRIBED {
                break;
            }
            if let Some(description) = self.describe(&snapshot, folder).await {
                self.parts
                    .repo
                    .set_description(snapshot.id, &description)
                    .await?;
                described += 1;
            }
        }
        Ok(described)
    }

    /// One snapshot's description, unless it shows a secret or fails.
    async fn describe(&self, snapshot: &Snapshot, folder: &Path) -> Option<String> {
        if self.shows_secret(snapshot).await {
            log::info!(target: TARGET, "snapshot {} shows hidden information; not described", snapshot.id);
            return None;
        }
        let png = tokio::fs::read(folder.join(&snapshot.file_name)).await.ok()?;
        match self.parts.describer.describe(png).await {
            Ok(text) => Some(clean(&text)),
            Err(error) => {
                log::warn!(target: TARGET, "snapshot {}: {error}", snapshot.id);
                None
            }
        }
    }

    /// Whether NOT-09 would hide part of the snapshot's text.
    async fn shows_secret(&self, snapshot: &Snapshot) -> bool {
        let Some(text) = snapshot.text.clone().filter(|text| !text.is_empty()) else {
            return false;
        };
        let redacted = self.parts.redactor.redact(vec![text.clone()]).await;
        redacted.first() != Some(&text)
    }
}

/// `text` trimmed, blank lines dropped, cut to [`MAX_TEXT_CHARS`].
pub fn clean(text: &str) -> String {
    let joined = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    joined.chars().take(MAX_TEXT_CHARS).collect()
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
