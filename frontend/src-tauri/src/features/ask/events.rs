//! An answer's pieces, sent over the `tauri::ipc::Channel` the UI passes to
//! each `ask` or `preview` request (PARITY_PLAN §4.0: per-request channels,
//! not global events).

use deino_llm::TextSink;
use serde::Serialize;
use tauri::ipc::Channel;

/// One step of a streamed answer.
#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AskStreamEvent {
    /// The next piece of the answer's text, `[s12]` markers and all; the
    /// command's result carries the resolved citations.
    Text {
        /// The piece.
        delta: String,
    },
}

/// Forwards an answer's pieces to the UI's channel.
pub(super) struct ChannelSink(pub Channel<AskStreamEvent>);

impl TextSink for ChannelSink {
    fn text(&self, piece: &str) {
        let event = AskStreamEvent::Text {
            delta: piece.to_owned(),
        };
        // A closed panel just misses the rest of the stream; the answer is
        // still saved.
        if let Err(error) = self.0.send(event) {
            log::debug!(target: "ask", "a stream channel closed: {error}");
        }
    }
}
