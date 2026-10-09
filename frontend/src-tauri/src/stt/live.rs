//! The live lane during a recording.
//!
//! The legacy pipeline mixes microphone and system audio and hands every
//! mixed window to a [`LiveAudioSink`] (continuous audio, never VAD-gated).
//! [`LiveTranscription`] resamples it to 16 kHz once and feeds the lane's
//! `LiveSession`, whose text reaches the UI through [`sink::TauriLiveSink`].
//! CAP-02 splits this into a microphone and a system lane.

pub mod sink;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use deino_stt::clock::MonotonicClock;
use deino_stt::perf::PerfReport;
use deino_stt::{
    EngineSupervisor, LiveSession, Millis, PinGuard, SessionConfig, SessionSink, SttError,
};

use super::resample::Downsampler;
use super::status::JobGuard;

/// Receives the pipeline's continuous mixed audio.
pub trait LiveAudioSink: Send + Sync {
    /// Takes one window of mono audio at `sample_rate`. Never blocks for long.
    fn push(&self, samples: &[f32], sample_rate: u32);
}

/// What a live lane is started with.
pub struct LiveStart {
    /// The lane's engine, loaded.
    pub engine: Arc<EngineSupervisor>,
    /// Where its text goes.
    pub sink: Box<dyn SessionSink>,
    /// Measures latency and real-time factor.
    pub clock: Arc<dyn MonotonicClock>,
    /// Keeps the engine loaded for the recording.
    pub pin: PinGuard,
    /// Marks transcription busy until the lane finishes.
    pub job: JobGuard,
}

/// One running live lane.
pub struct LiveTranscription {
    session: Mutex<Option<LiveSession>>,
    downsampler: Mutex<Option<(u32, Downsampler)>>,
    held: Mutex<Option<(PinGuard, JobGuard)>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl LiveTranscription {
    /// Opens the lane's stream and starts its session.
    pub fn start(start: LiveStart) -> Result<Self, SttError> {
        let config = SessionConfig {
            label: deino_stt::Lane::Mixed.as_str(),
            ..SessionConfig::default()
        };
        let session = LiveSession::start(start.engine, start.sink, start.clock, config)?;
        Ok(Self {
            session: Mutex::new(Some(session)),
            downsampler: Mutex::new(None),
            held: Mutex::new(Some((start.pin, start.job))),
        })
    }

    /// Audio waiting for the engine.
    pub fn backlog(&self) -> Millis {
        lock(&self.session)
            .as_ref()
            .map_or(Millis::ZERO, LiveSession::backlog)
    }

    /// Transcribes everything fed so far and stops the lane. Blocks until
    /// the last segment is out; later pushes are ignored.
    pub fn finish(&self) -> Option<PerfReport> {
        let session = lock(&self.session).take()?;
        let report = session.finish();
        lock(&self.held).take();
        Some(report)
    }

    /// Resamples `samples` to 16 kHz, recreating the resampler on a rate change.
    fn resample(&self, samples: &[f32], sample_rate: u32) -> Vec<f32> {
        let mut slot = lock(&self.downsampler);
        if slot.as_ref().is_none_or(|(rate, _)| *rate != sample_rate) {
            match Downsampler::new(sample_rate) {
                Ok(downsampler) => *slot = Some((sample_rate, downsampler)),
                Err(error) => {
                    log::error!(target: "stt", "cannot resample {sample_rate} Hz audio: {error}");
                    return Vec::new();
                }
            }
        }
        slot.as_mut()
            .map(|(_, downsampler)| downsampler.push(samples))
            .unwrap_or_default()
    }
}

impl LiveAudioSink for LiveTranscription {
    fn push(&self, samples: &[f32], sample_rate: u32) {
        let pcm = self.resample(samples, sample_rate);
        if pcm.is_empty() {
            return;
        }
        if let Some(session) = lock(&self.session).as_ref() {
            session.feed(pcm);
        }
    }
}
