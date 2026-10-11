//! `StreamingLane`: a lane streamed through `deino_stt::LiveSession`.

use std::sync::Arc;

use deino_stt::clock::MonotonicClock;
use deino_stt::{LiveSession, Millis, SessionConfig, SessionSink, Transcriber};

use super::{EngineError, LaneTranscriber};
use crate::lane::Lane;

/// What a streaming lane is built from.
pub struct StreamingParts {
    /// The lane.
    pub lane: Lane,
    /// The lane's engine (a loaded worker in the app, a fake in tests).
    pub transcriber: Arc<dyn Transcriber>,
    /// Where text goes.
    pub sink: Box<dyn SessionSink>,
    /// Measures latency for the lane's perf log.
    pub clock: Arc<dyn MonotonicClock>,
    /// Anything that must live as long as the lane (the engine's pin).
    pub keep_alive: Vec<Box<dyn Send>>,
}

/// A lane transcribed by a live stream.
pub struct StreamingLane {
    lane: Lane,
    session: LiveSession,
    _keep_alive: Vec<Box<dyn Send>>,
}

impl StreamingLane {
    /// Opens the stream (so a missing model fails here, not mid-meeting).
    pub fn start(parts: StreamingParts) -> Result<Self, EngineError> {
        let config = SessionConfig {
            label: parts.lane.engine().as_str(),
            ..SessionConfig::default()
        };
        let session = LiveSession::start(parts.transcriber, parts.sink, parts.clock, config)
            .map_err(|source| EngineError {
                lane: parts.lane,
                source: Box::new(source),
            })?;
        Ok(Self {
            lane: parts.lane,
            session,
            _keep_alive: parts.keep_alive,
        })
    }
}

impl LaneTranscriber for StreamingLane {
    fn feed(&mut self, pcm: Vec<f32>) {
        self.session.feed(pcm);
    }

    fn backlog(&self) -> Millis {
        self.session.backlog()
    }

    fn finish(self: Box<Self>) {
        let Self { lane, session, .. } = *self;
        let report = session.finish();
        log::info!(
            target: "recording",
            "{} lane finished: audio {:.1} s, RTF {:.3}, first text p50 {:?} ms, {} recoveries",
            lane.as_str(),
            report.audio_s,
            report.rtf,
            report.first_text_p50_ms,
            report.recoveries
        );
    }
}
