//! Delivers a live lane's text to the UI.
//!
//! The growing segment goes out as the typed `LiveTranscript` event; a closed
//! segment goes out as the legacy `transcript-update` event, which the record
//! screen lists and the stop flow saves (until FND-05b streams segments,
//! with their timed words, into `transcript_segments` from Rust). The first
//! text of a recording also fires the legacy `speech-detected` event.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use deino_domain::Clock;
use deino_stt::{FinalSegment, Lane, LiveText, SessionSink};
use tauri::{AppHandle, Emitter, Runtime};
use tauri_specta::Event;

use crate::audio::transcription::TranscriptUpdate;
use crate::features::transcription::LiveTranscript;
use crate::stt::status::TranscriptionActivity;

/// Confidence reported for a segment whose words carry none.
const DEFAULT_CONFIDENCE: f32 = 0.85;

/// Emits a lane's text as Tauri events.
pub struct TauriLiveSink<R: Runtime> {
    app: AppHandle<R>,
    lane: Lane,
    clock: Arc<dyn Clock>,
    /// Shared by every recording, so sequence ids never repeat in a session.
    sequence: Arc<AtomicU64>,
    activity: Arc<TranscriptionActivity>,
    speech_announced: bool,
}

/// What a sink is built from.
pub struct SinkParts<R: Runtime> {
    /// Emits the events.
    pub app: AppHandle<R>,
    /// The lane.
    pub lane: Lane,
    /// Stamps the wall-clock time shown next to a line.
    pub clock: Arc<dyn Clock>,
    /// The next `transcript-update` sequence id.
    pub sequence: Arc<AtomicU64>,
    /// Records that transcription is active.
    pub activity: Arc<TranscriptionActivity>,
}

impl<R: Runtime> TauriLiveSink<R> {
    /// A sink for one recording's lane.
    pub fn new(parts: SinkParts<R>) -> Self {
        Self {
            app: parts.app,
            lane: parts.lane,
            clock: parts.clock,
            sequence: parts.sequence,
            activity: parts.activity,
            speech_announced: false,
        }
    }

    fn announce_speech(&mut self) {
        if self.speech_announced {
            return;
        }
        self.speech_announced = true;
        let message = serde_json::json!({ "message": "Speech activity detected" });
        if let Err(error) = self.app.emit("speech-detected", message) {
            log::debug!(target: "stt", "could not emit speech-detected: {error}");
        }
    }

    fn update(&self, segment: &FinalSegment) -> TranscriptUpdate {
        let start = segment.start.as_secs_f64();
        let end = segment.end.as_secs_f64();
        TranscriptUpdate {
            text: segment.text.clone(),
            timestamp: self.clock.now().format("%H:%M:%S").to_string(),
            source: "Audio".to_string(),
            sequence_id: self.sequence.fetch_add(1, Ordering::SeqCst),
            chunk_start_time: start,
            is_partial: false,
            confidence: confidence(segment),
            audio_start_time: start,
            audio_end_time: end,
            duration: end - start,
        }
    }
}

/// The segment's mean word confidence, or the default.
#[allow(
    clippy::cast_precision_loss,
    reason = "a segment has at most a few hundred words, far below f32's exact range"
)]
fn confidence(segment: &FinalSegment) -> f32 {
    let scores: Vec<f32> = segment.words.iter().filter_map(|w| w.confidence).collect();
    if scores.is_empty() {
        DEFAULT_CONFIDENCE
    } else {
        scores.iter().sum::<f32>() / scores.len() as f32
    }
}

impl<R: Runtime> SessionSink for TauriLiveSink<R> {
    fn live(&mut self, text: &LiveText) {
        self.activity.touch();
        if !text.committed.is_empty() {
            self.announce_speech();
        }
        let event = LiveTranscript {
            lane: self.lane.into(),
            committed: text.committed.clone(),
            tentative: text.tentative.clone(),
            start_seconds: text.start.as_secs_f64(),
        };
        if let Err(error) = event.emit(&self.app) {
            log::debug!(target: "stt", "could not emit live-transcript: {error}");
        }
    }

    fn segment(&mut self, segment: FinalSegment) {
        self.activity.touch();
        self.announce_speech();
        let update = self.update(&segment);
        log::debug!(
            target: "stt",
            "segment {} on lane {}: {} words, {:.1}-{:.1} s ({:?})",
            update.sequence_id,
            self.lane.as_str(),
            segment.words.len(),
            update.audio_start_time,
            update.audio_end_time,
            segment.origin
        );
        if let Err(error) = self.app.emit("transcript-update", &update) {
            log::warn!(target: "stt", "could not emit transcript-update: {error}");
        }
    }
}
