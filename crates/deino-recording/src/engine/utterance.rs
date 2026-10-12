//! `UtteranceLane`: a lane transcribed one voice span at a time.
//!
//! The "Advanced" engines (Parakeet TDT over ONNX, Whisper) cannot stream.
//! The lane keeps the last [`HISTORY`] of audio; when its voice detector
//! closes a span (or a span passes [`MAX_UTTERANCE`]), the span plus
//! [`PADDING`] on each side is transcribed on a worker thread and delivered
//! as one segment timed by the span. Segments carry no word timings.
//! After each feed the lane reports its transcript watermark: the start of
//! the open span, or everything fed when nobody is speaking, queued behind
//! the jobs it covers so it always follows their text.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;

use deino_stt::{FinalSegment, Millis, SegmentOrigin, SessionSink};

use super::{EngineError, LaneRequest, LaneTranscriber};

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

/// What the worker thread does next, in order.
enum Work {
    /// Transcribe an utterance.
    Utterance(Job),
    /// Report the watermark: every utterance before it has been delivered.
    Progress(Millis),
}

/// A lane transcribed by voice span.
pub struct UtteranceLane {
    audio: VecDeque<f32>,
    /// The 16 kHz position of `audio`'s first sample.
    audio_start: usize,
    speech_start: Option<Millis>,
    reported: Millis,
    jobs: Option<Sender<Work>>,
    worker: Option<JoinHandle<()>>,
}

impl UtteranceLane {
    /// Starts the lane's worker thread.
    pub fn start(
        request: LaneRequest,
        transcriber: Arc<dyn UtteranceTranscriber>,
    ) -> Result<Self, EngineError> {
        let LaneRequest {
            lane,
            mut sink,
            start,
        } = request;
        let (jobs, queue) = channel::<Work>();
        let worker = std::thread::Builder::new()
            .name(format!("deino-utterance-{}", lane.as_str()))
            .spawn(move || serve(&queue, transcriber.as_ref(), sink.as_mut()))
            .map_err(|source| EngineError {
                lane,
                source: Box::new(source),
            })?;
        Ok(Self {
            audio: VecDeque::with_capacity(Millis(HISTORY.as_secs() * 1_000).to_samples()),
            audio_start: start.to_samples(),
            speech_start: None,
            reported: start,
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
        self.send(Work::Utterance(Job { start, end, pcm }));
    }

    /// Queues the watermark when it moved: the open span's start, or all
    /// the audio while nobody speaks.
    fn report_progress(&mut self) {
        let through = self.speech_start.unwrap_or_else(|| self.position());
        if through > self.reported {
            self.reported = through;
            self.send(Work::Progress(through));
        }
    }

    fn send(&self, work: Work) {
        if let Some(jobs) = &self.jobs {
            jobs.send(work).ok();
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
        self.report_progress();
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
        self.report_progress();
        self.jobs = None;
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            log::error!(target: "recording", "an utterance worker panicked");
        }
    }
}

/// The worker thread: utterances and watermarks, in the order queued.
fn serve(
    queue: &std::sync::mpsc::Receiver<Work>,
    transcriber: &dyn UtteranceTranscriber,
    sink: &mut dyn SessionSink,
) {
    for work in queue {
        match work {
            Work::Utterance(job) => transcribe(transcriber, sink, job),
            Work::Progress(through) => sink.progress(through),
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
    use crate::lane::Lane;
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

    /// What the lane delivered, in order.
    #[derive(Clone, Debug, PartialEq)]
    enum Seen {
        Segment(Millis, Millis, String),
        Progress(Millis),
    }

    #[derive(Clone, Default)]
    struct Collect(Arc<Mutex<Vec<Seen>>>);

    impl Collect {
        fn segments(&self) -> Vec<(Millis, Millis, String)> {
            let seen = self.0.lock().unwrap();
            seen.iter()
                .filter_map(|seen| match seen {
                    Seen::Segment(start, end, text) => Some((*start, *end, text.clone())),
                    Seen::Progress(_) => None,
                })
                .collect()
        }
    }

    impl SessionSink for Collect {
        fn live(&mut self, _text: &LiveText) {}
        fn segment(&mut self, segment: FinalSegment) {
            let seen = Seen::Segment(segment.start, segment.end, segment.text);
            self.0.lock().unwrap().push(seen);
        }
        fn progress(&mut self, through: Millis) {
            self.0.lock().unwrap().push(Seen::Progress(through));
        }
    }

    fn lane(lane: Lane, collect: &Collect, start: Millis) -> UtteranceLane {
        let request = LaneRequest {
            lane,
            sink: Box::new(collect.clone()),
            start,
        };
        UtteranceLane::start(request, Arc::new(Echo)).unwrap()
    }

    #[test]
    fn a_closed_voice_span_becomes_one_padded_segment() {
        let collect = Collect::default();
        let mut lane = Box::new(lane(Lane::Me, &collect, Millis::ZERO));
        lane.feed(vec![0.1; 48_000]);
        lane.voice(VoiceChange::Started(Millis(1_000)));
        let span = VoiceSpan {
            start: Millis(1_000),
            end: Millis(2_000),
        };
        lane.voice(VoiceChange::Ended(span));
        lane.finish();

        let segments = collect.segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(
            (segments[0].0, segments[0].1),
            (Millis(1_000), Millis(2_000))
        );
        assert_eq!(segments[0].2, "22400 samples");
    }

    #[test]
    fn a_long_span_is_cut_at_the_utterance_limit() {
        let collect = Collect::default();
        let mut lane = Box::new(lane(Lane::Them, &collect, Millis::ZERO));
        lane.voice(VoiceChange::Started(Millis::ZERO));
        for _ in 0..30 {
            lane.feed(vec![0.1; 16_000]);
        }
        lane.finish();

        let segments = collect.segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].1, Millis(25_000));
        assert_eq!(segments[1].1, Millis(30_000));
    }

    #[test]
    fn the_watermark_stops_at_open_speech_and_follows_the_segment_it_covers() {
        let collect = Collect::default();
        let mut lane = Box::new(lane(Lane::Them, &collect, Millis(60_000)));
        lane.feed(vec![0.1; 16_000]);
        lane.voice(VoiceChange::Started(Millis(61_000)));
        lane.feed(vec![0.1; 16_000]);
        lane.voice(VoiceChange::Ended(VoiceSpan {
            start: Millis(61_000),
            end: Millis(61_800),
        }));
        lane.feed(vec![0.1; 16_000]);
        lane.finish();

        let seen = collect.0.lock().unwrap().clone();
        assert_eq!(
            seen,
            [
                Seen::Progress(Millis(61_000)),
                Seen::Segment(Millis(61_000), Millis(61_800), "19200 samples".into()),
                Seen::Progress(Millis(63_000)),
            ]
        );
    }
}
