//! The session thread: feeds the stream, assembles segments, recovers.

mod recovery;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use super::assembler::{Assembler, Closed};
use super::retained::Retained;
use super::{
    FinalSegment, Input, LiveText, SegmentOrigin, SessionConfig, SessionSink, SharedStatus,
};
use crate::clock::MonotonicClock;
use crate::pcm::Millis;
use crate::perf::{FeedSample, PerfReport, StreamPerf};
use crate::protocol::StreamDelta;
use crate::transcriber::{Transcriber, TranscriptStream};
use crate::words::{TimedToken, text_of, words_from_tokens};

/// At most this much queued audio is fed in one call when the lane is behind.
const MAX_FEED: Millis = Millis(2000);

/// Without a stream, retained audio is transcribed in one run (and a new
/// stream tried) once it reaches this length.
const STREAMLESS_BATCH: Millis = Millis(15_000);

/// What a runner is built from.
pub(super) struct Parts {
    pub transcriber: Arc<dyn Transcriber>,
    pub stream: Box<dyn TranscriptStream>,
    pub sink: Box<dyn SessionSink>,
    pub clock: Arc<dyn MonotonicClock>,
    pub config: SessionConfig,
    pub queued: Arc<AtomicUsize>,
    pub status: Arc<SharedStatus>,
}

/// The lane's state, owned by its thread.
pub(super) struct Runner {
    transcriber: Arc<dyn Transcriber>,
    stream: Option<Box<dyn TranscriptStream>>,
    sink: Box<dyn SessionSink>,
    clock: Arc<dyn MonotonicClock>,
    config: SessionConfig,
    queued: Arc<AtomicUsize>,
    status: Arc<SharedStatus>,
    /// The transcript watermark last reported ([`SessionSink::progress`]).
    reported: Millis,
    /// Timeline position where the current stream began.
    stream_start: Millis,
    /// Samples fed so far, from the session start.
    fed: usize,
    retained: Retained,
    assembler: Assembler,
    perf: StreamPerf,
    live: LiveText,
}

impl Runner {
    pub(super) fn new(parts: Parts) -> Self {
        let Parts {
            transcriber,
            stream,
            sink,
            clock,
            config,
            queued,
            status,
        } = parts;
        let perf = StreamPerf::new(config.label, config.perf_interval, clock.now());
        Self {
            transcriber,
            stream: Some(stream),
            sink,
            clock,
            queued,
            status,
            reported: config.start,
            stream_start: config.start,
            fed: 0,
            retained: Retained::at(config.start),
            assembler: Assembler::new(config.policy),
            perf,
            live: LiveText::default(),
            config,
        }
    }

    /// Serves audio until `Finish`, then flushes everything.
    pub(super) fn run(mut self, input: &Receiver<Input>) -> PerfReport {
        while let Ok(Input::Audio(pcm)) = input.recv() {
            let (pcm, finished) = coalesce(pcm, input);
            self.queued.fetch_sub(pcm.len(), Ordering::Relaxed);
            self.on_audio(pcm);
            if finished {
                break;
            }
        }
        self.finish()
    }

    /// The timeline position at the end of the fed audio.
    fn position(&self) -> Millis {
        self.config.start + Millis::from_samples(self.fed)
    }

    fn on_audio(&mut self, pcm: Vec<f32>) {
        self.retained.push(&pcm);
        self.fed += pcm.len();
        let audio = Millis::from_samples(pcm.len());
        let Some(stream) = self.stream.as_mut() else {
            if self.retained.duration() >= STREAMLESS_BATCH {
                self.recover(true);
            }
            return;
        };
        let started = self.clock.now();
        match stream.feed(pcm) {
            Ok(delta) => {
                let compute = self.clock.now().saturating_sub(started);
                self.apply(&delta, audio, compute);
                self.rotate_if_due();
            }
            Err(error) => {
                log::warn!(
                    "lane {}: {error}; re-transcribing its retained audio",
                    self.config.label
                );
                self.recover(true);
            }
        }
    }

    /// Applies a stream delta: segments, trimming, live text and metrics.
    fn apply(&mut self, delta: &StreamDelta, audio: Millis, compute: std::time::Duration) {
        let tokens = on_timeline(&delta.tokens, self.stream_start);
        let token_ends: Vec<i64> = tokens.iter().map(|token| token.end_ms).collect();
        let pushed = self.assembler.push(tokens);
        for closed in pushed.closed {
            self.emit(&closed, SegmentOrigin::Stream);
        }
        let decoded = self.stream_start + Millis::from_engine(delta.progress.committed_ms);
        if let Some(closed) = self.assembler.advance(decoded) {
            self.emit(&closed, SegmentOrigin::Stream);
        }
        let open_start = self
            .assembler
            .open()
            .first()
            .map(|token| Millis::from_engine(token.start_ms));
        self.retained.trim_before(open_start.unwrap_or(decoded));
        self.publish_live(delta.tentative.as_deref());
        self.report_progress(decoded);
        let pending = Millis::from_engine(delta.progress.pending_ms());
        let queued = Millis::from_samples(self.queued.load(Ordering::Relaxed));
        self.perf.feed(&FeedSample {
            now: self.clock.now(),
            fed_to: self.position(),
            audio,
            compute,
            pending,
            queued,
            token_ends: &token_ends,
            onsets: &pushed.opened,
        });
    }

    fn emit(&mut self, closed: &Closed, origin: SegmentOrigin) {
        let Some(segment) = final_segment(&closed.0, origin) else {
            return;
        };
        self.retained.trim_before(segment.end);
        self.perf.segment();
        self.sink.segment(segment);
    }

    /// Reports the transcript watermark when it moved forward.
    pub(super) fn report_progress(&mut self, through: Millis) {
        if through > self.reported {
            self.reported = through;
            self.sink.progress(through);
        }
    }

    /// Sends the open segment's text when it changed.
    fn publish_live(&mut self, tentative: Option<&str>) {
        let open = self.assembler.open();
        let live = LiveText {
            start: open
                .first()
                .map_or(Millis::ZERO, |token| Millis::from_engine(token.start_ms)),
            committed: text_of(open),
            tentative: tentative.map_or_else(|| self.live.tentative.clone(), str::to_string),
        };
        if live != self.live {
            self.sink.live(&live);
            self.live = live;
        }
    }
}

/// Appends queued audio to `pcm`, up to [`MAX_FEED`]; true when `Finish`
/// arrived (after all audio, which the FIFO guarantees).
fn coalesce(mut pcm: Vec<f32>, input: &Receiver<Input>) -> (Vec<f32>, bool) {
    while pcm.len() < MAX_FEED.to_samples() {
        match input.try_recv() {
            Ok(Input::Audio(more)) => pcm.extend(more),
            Ok(Input::Finish) => return (pcm, true),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
        }
    }
    (pcm, false)
}

/// Shifts stream-relative tokens onto the recording's timeline.
fn on_timeline(tokens: &[TimedToken], start: Millis) -> Vec<TimedToken> {
    let offset = i64::try_from(start.0).unwrap_or(i64::MAX);
    tokens
        .iter()
        .map(|token| TimedToken {
            start_ms: token.start_ms.saturating_add(offset),
            end_ms: token.end_ms.saturating_add(offset),
            ..token.clone()
        })
        .collect()
}

fn final_segment(tokens: &[TimedToken], origin: SegmentOrigin) -> Option<FinalSegment> {
    let words = words_from_tokens(tokens, Millis::ZERO);
    let first = words.first()?;
    let end = words.iter().map(|word| word.end).max().unwrap_or(first.end);
    Some(FinalSegment {
        start: first.start,
        end,
        text: text_of(tokens),
        words,
        origin,
    })
}
