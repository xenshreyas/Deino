//! `LaneInput`: one lane's voice detector and transcriber feed.
//!
//! The pipeline hands each lane aligned 10 ms frames. The lane scores them
//! for voice (the Me lane gets the echo-cancelled microphone), tells the
//! transcriber and the transcript router where speech starts and ends, and
//! feeds the transcriber continuous audio in [`FEED`]-sample batches.

use deino_stt::Millis;

use crate::engine::LaneTranscriber;
use crate::lane::Lane;
use crate::transcript::RouterHandle;
use crate::vad::LaneVad;

/// Samples per transcriber feed: 100 ms at 16 kHz.
pub const FEED: usize = 1_600;

/// One lane's input side.
pub struct LaneInput {
    lane: Lane,
    vad: LaneVad,
    transcriber: Box<dyn LaneTranscriber>,
    router: RouterHandle,
    batch: Vec<f32>,
}

impl LaneInput {
    /// A lane feeding `transcriber` and reporting voice to `router`.
    pub fn new(lane: Lane, transcriber: Box<dyn LaneTranscriber>, router: RouterHandle) -> Self {
        Self {
            lane,
            vad: LaneVad::default(),
            transcriber,
            router,
            batch: Vec::with_capacity(FEED),
        }
    }

    /// Takes the lane's next frame.
    pub fn push(&mut self, frame: &[f32]) {
        let (transcriber, router, lane) = (&mut self.transcriber, &self.router, self.lane);
        self.vad.push(frame, |change| {
            transcriber.voice(change);
            router.voice(lane, change);
        });
        self.batch.extend_from_slice(frame);
        if self.batch.len() >= FEED {
            let batch = std::mem::replace(&mut self.batch, Vec::with_capacity(FEED));
            self.transcriber.feed(batch);
        }
    }

    /// Audio fed and not yet transcribed.
    pub fn backlog(&self) -> Millis {
        self.transcriber.backlog()
    }

    /// Flushes the last batch and open voice span, then finishes the
    /// transcriber (blocking until its last segment is delivered).
    pub fn finish(mut self) {
        if !self.batch.is_empty() {
            self.transcriber.feed(std::mem::take(&mut self.batch));
        }
        if let Some(change) = self.vad.finish() {
            self.transcriber.voice(change);
            self.router.voice(self.lane, change);
        }
        self.transcriber.finish();
    }
}
