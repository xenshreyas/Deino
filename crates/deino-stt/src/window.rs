//! The Parakeet Unified streaming window: how much audio the encoder sees.
//!
//! A buffered Parakeet stream decodes `chunk` ms at a time, looking `left` ms
//! back and `right` ms ahead, so text for a word lands roughly `chunk + right`
//! after it is spoken. The model's default (5600/1040/1040 ms) gives about
//! 2.1 s to first text. The STT spike swept the model's training menu on an
//! M4 Max (`deino-spikes/stt-streaming/results/sweep_table.md`):
//!
//! | C/R (ms) | conv WER | onset latency p50/p95 | stream RTF |
//! |---|---|---|---|
//! | 1040/1040 (default) | 0.40% | 1979/2516 ms | 0.035 |
//! | 560/560 | 0.40% | 1169/1506 ms | 0.064 |
//! | **560/320** | 0.81% | 932/1285 ms | 0.060 |
//! | 160/320 | 0.81% | 758/1000 ms | 0.251 |
//!
//! [`StreamWindow::LIVE`] (5600/560/320) meets the ~1.5 s target with the
//! accuracy of the default and a quarter of the 160 ms chunk's compute.

use serde::{Deserialize, Serialize};

/// Window sizes must be whole encoder frames.
const FRAME_MS: u32 = 80;

/// A buffered streaming window, in milliseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamWindow {
    /// Left context, in ms.
    left: u32,
    /// Chunk size, in ms.
    chunk: u32,
    /// Lookahead, in ms.
    right: u32,
}

/// A window whose sizes are not whole 80 ms encoder frames, or whose chunk is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("stream window sizes must be multiples of {FRAME_MS} ms with a non-empty chunk")]
pub struct InvalidWindow;

impl StreamWindow {
    /// The live window Deino uses: 5600 ms left context, 560 ms chunks,
    /// 320 ms lookahead (see the module docs for the measurements).
    pub const LIVE: Self = Self {
        left: 5600,
        chunk: 560,
        right: 320,
    };

    /// A window of `left`, `chunk` and `right` milliseconds.
    pub fn new(left_ms: u32, chunk_ms: u32, right_ms: u32) -> Result<Self, InvalidWindow> {
        let whole = [left_ms, chunk_ms, right_ms]
            .iter()
            .all(|ms| ms % FRAME_MS == 0);
        if !whole || chunk_ms == 0 {
            return Err(InvalidWindow);
        }
        Ok(Self {
            left: left_ms,
            chunk: chunk_ms,
            right: right_ms,
        })
    }

    /// Left context in ms.
    pub fn left_ms(self) -> u32 {
        self.left
    }

    /// Chunk size in ms.
    pub fn chunk_ms(self) -> u32 {
        self.chunk
    }

    /// Lookahead in ms.
    pub fn right_ms(self) -> u32 {
        self.right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_live_window_is_a_valid_window() {
        assert_eq!(StreamWindow::new(5600, 560, 320), Ok(StreamWindow::LIVE));
    }

    #[test]
    fn sizes_off_the_encoder_frame_grid_are_rejected() {
        assert_eq!(StreamWindow::new(5600, 500, 320), Err(InvalidWindow));
        assert_eq!(StreamWindow::new(5600, 0, 320), Err(InvalidWindow));
    }
}
