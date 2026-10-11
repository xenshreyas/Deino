//! `UtteranceLane`: a lane transcribed one voice span at a time.
//!
//! The "Advanced" engines (Parakeet TDT over ONNX, Whisper) cannot stream.
//! The lane keeps the last [`HISTORY`] of audio; when its voice detector
//! closes a span (or a span passes [`MAX_UTTERANCE`]), the span plus
//! [`PADDING`] on each side is transcribed on a worker thread and delivered
//! as one segment timed by the span. Segments carry no word timings.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_stt::{FinalSegment, Millis, SegmentOrigin, SessionSink};

use super::{EngineError, LaneTranscriber};
use crate::lane::Lane;
use crate::vad::VoiceChange;

/// Audio kept for slicing spans out of.
pub const HISTORY: Duration = Duration::from_mins(1);

/// A span this long is cut and transcribed even while speech continues
/// (the legacy engines' 25 s limit).
pub const MAX_UTTERANCE: Millis = Millis(25_000);

/// Context added before and after a span.
pub const PADDING: Millis = Millis(200);

/// Batch transcription of one utterance.
pub trait UtteranceTranscriber: Send + Sync {
    /// The text of 16 kHz mono `pcm`.
    fn transcribe(&self, pcm: Vec<f32>)
    -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

/// One utterance to transcribe.
struct Job {
    start: Millis,
    end: Millis,
    pcm: Vec<f32>,
}

/// A lane transcribed by voice span.
pub struct UtteranceLane {
    audio: VecDeque<f32>,
    /// The 16 kHz position of `audio`'s first sample.
    audio_start: usize,
    speech_start: Option<Millis>,
    jobs: Option<Sender<Job>>,
    worker: Option<JoinHandle<()>>,
}

impl UtteranceLane {
    /// Starts the lane's worker thread.
    pub fn start(
        lane: Lane,
        transcriber: Arc<dyn UtteranceTranscriber>,
        mut sink: Box<dyn SessionSink>,
    ) -> Result<Self, EngineError> {
        let (jobs, queue) = channel::<Job>();
        let worker = std::thread::Builder::new()
            .name(format!("deino-utterance-{}", lane.as_str()))
            .spawn(move || {
                for job in queue {
                    transcribe(transcriber.as_ref(), sink.as_mut(), job);
                }
            })
            .map_err(|source| EngineError {
                lane,
                source: Box::new(source),
            })?;
        Ok(Self {
            audio: VecDeque::with_capacity(Millis(HISTORY.as_secs() * 1_000).to_samples()),
            audio_start: 0,
            speech_start: None,
            jobs: Some(jobs),
            worker: Some(worker),
        })
    }

    /// Queues `[start, end]` (plus padding) for transcription.
    fn cut(&mut self, start: Millis, end: Millis) {
        let from = start
            .saturating_sub(PADDING)
            .to_samples()
            .max(self.audio_start);
        let to = (end + PADDING).to_samples();
        let pcm: Vec<f32> = self
            .audio
            .iter()
            .skip(from - self.audio_start)
            .take(to.saturating_sub(from))
            .copied()
            .collect();
        if let Some(jobs) = &self.jobs {
            jobs.send(Job { start, end, pcm }).ok();
        }
    }

    fn position(&self) -> Millis {
        Millis::from_samples(self.audio_start + self.audio.len())
    }
}

impl LaneTranscriber for UtteranceLane {
    fn feed(&mut self, pcm: Vec<f32>) {
        self.audio.extend(pcm);
        let keep = Millis(HISTORY.as_secs() * 1_000).to_samples();
        let excess = self.audio.len().saturating_sub(keep);
        self.audio.drain(..excess);
        self.audio_start += excess;
        let now = self.position();
        if let Some(start) = self.speech_start
            && now.saturating_sub(start) >= MAX_UTTERANCE
        {
            self.cut(start, now);
            self.speech_start = Some(now);
        }
    }

    fn voice(&mut self, change: VoiceChange) {
        match change {
            VoiceChange::Started(at) => self.speech_start = Some(at),
            VoiceChange::Ended(span) => {
                let start = self
                    .speech_start
                    .take()
                    .unwrap_or(span.start)
                    .max(span.start);
                self.cut(start, span.end);
            }
        }
    }

    fn finish(mut self: Box<Self>) {
        if let Some(start) = self.speech_start.take() {
            let end = self.position();
            self.cut(start, end);
        }
        self.jobs = None;
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            log::error!(target: "recording", "an utterance worker panicked");
        }
    }
}

/// Transcribes one job and delivers its segment.
fn transcribe(transcriber: &dyn UtteranceTranscriber, sink: &mut dyn SessionSink, job: Job) {
    match transcriber.transcribe(job.pcm) {
        Ok(text) if !text.trim().is_empty() => sink.segment(FinalSegment {
            start: job.start,
            end: job.end,
            text: text.trim().to_owned(),
            words: Vec::new(),
            origin: SegmentOrigin::Stream,
        }),
        Ok(_) => {}
        Err(error) => log::warn!(target: "recording", "an utterance failed to transcribe: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use deino_stt::LiveText;

    use super::*;
    use crate::vad::VoiceSpan;

    struct Echo;

    impl UtteranceTranscriber for Echo {
        fn transcribe(
            &self,
            pcm: Vec<f32>,
        ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            Ok(format!("{} samples", pcm.len()))
        }
    }

    #[derive(Clone, Default)]
    struct Collect(Arc<Mutex<Vec<FinalSegment>>>);

    impl SessionSink for Collect {
        fn live(&mut self, _text: &LiveText) {}
        fn segment(&mut self, segment: FinalSegment) {
            self.0.lock().unwrap().push(segment);
        }
    }

    #[test]
    fn a_closed_voice_span_becomes_one_padded_segment() {
        let collect = Collect::default();
        let mut lane = Box::new(
            UtteranceLane::start(Lane::Me, Arc::new(Echo), Box::new(collect.clone())).unwrap(),
        );
        lane.feed(vec![0.1; 48_000]);
        lane.voice(VoiceChange::Started(Millis(1_000)));
        let span = VoiceSpan {
            start: Millis(1_000),
            end: Millis(2_000),
        };
        lane.voice(VoiceChange::Ended(span));
        lane.finish();

        let segments = collect.0.lock().unwrap().clone();
        assert_eq!(segments.len(), 1);
        assert_eq!(
            (segments[0].start, segments[0].end),
            (Millis(1_000), Millis(2_000))
        );
        assert_eq!(segments[0].text, "22400 samples");
    }

    #[test]
    fn a_long_span_is_cut_at_the_utterance_limit() {
        let collect = Collect::default();
        let mut lane = Box::new(
            UtteranceLane::start(Lane::Them, Arc::new(Echo), Box::new(collect.clone())).unwrap(),
        );
        lane.voice(VoiceChange::Started(Millis::ZERO));
        for _ in 0..30 {
            lane.feed(vec![0.1; 16_000]);
        }
        lane.finish();

        let segments = collect.0.lock().unwrap().clone();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].end, Millis(25_000));
        assert_eq!(segments[1].end, Millis(30_000));
    }
}
