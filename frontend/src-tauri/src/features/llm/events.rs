//! The `llm` plugin's streamed and broadcast messages.
//!
//! - [`LlmStreamEvent`]: one request's tokens, sent over the
//!   `tauri::ipc::Channel` the UI passes to that request (PARITY_PLAN §4.0:
//!   per-request channels, not global events).
//! - [`OllamaPull`]: progress of an Ollama model download, broadcast as
//!   `plugin:llm:ollama-pull`, because a download outlives the dialog that
//!   started it.

use deino_llm::{PullProgress, TextSink};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// One step of a streamed reply.
#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LlmStreamEvent {
    /// The next piece of the reply's text.
    Text {
        /// The piece.
        delta: String,
    },
}

/// Forwards a reply's pieces to the UI's channel.
pub(super) struct ChannelSink(pub Channel<LlmStreamEvent>);

impl TextSink for ChannelSink {
    fn text(&self, piece: &str) {
        let event = LlmStreamEvent::Text {
            delta: piece.to_owned(),
        };
        // A closed window just misses the rest of the reply.
        if let Err(error) = self.0.send(event) {
            log::debug!(target: "llm", "a stream channel closed: {error}");
        }
    }
}

/// How an Ollama model download is going.
#[derive(Clone, Debug, Serialize, specta::Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct OllamaPull {
    /// The model being downloaded.
    pub model: String,
    /// Its progress, or why it failed.
    pub status: OllamaPullStatus,
}

/// An [`OllamaPull`]'s state.
#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OllamaPullStatus {
    /// Downloading.
    Progress {
        /// 0 to 100.
        percent: u8,
    },
    /// The model is ready.
    Done,
    /// The download failed.
    Failed {
        /// A user-safe reason.
        message: String,
    },
}

impl From<PullProgress> for OllamaPullStatus {
    fn from(progress: PullProgress) -> Self {
        match progress {
            PullProgress::Downloading { percent } => Self::Progress { percent },
            PullProgress::Done => Self::Done,
        }
    }
}

/// Where pull updates go (managed as `Arc<dyn PullEvents>`).
pub trait PullEvents: Send + Sync {
    /// Announces `status` for `model`.
    fn emit(&self, model: &str, status: OllamaPullStatus);
}

/// Broadcasts pull updates as `plugin:llm:ollama-pull`.
pub struct TauriPullEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriPullEvents<R> {
    /// An adapter that emits through `app`.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> PullEvents for TauriPullEvents<R> {
    fn emit(&self, model: &str, status: OllamaPullStatus) {
        let event = OllamaPull {
            model: model.to_owned(),
            status,
        };
        // A missed update only delays a progress bar.
        if let Err(error) = event.emit(&self.app) {
            log::warn!(target: "llm", "could not emit ollama-pull: {error}");
        }
    }
}
