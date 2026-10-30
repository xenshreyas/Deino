//! Transcribing a finished track after the meeting (CAP-08): the
//! `final_transcription` stage's engine.
//!
//! [`OfflineTranscriber`] decodes the track to 16 kHz ([`TrackDecoder`]),
//! finds its utterances with the lanes' voice detector ([`utterances`]), and
//! runs the `deino-stt` batch engine ([`BatchEngines`]) on each, with word
//! times placed on the recording's timeline. Batch decoding can stop early on
//! some clips (the STT spike), so an utterance that comes back with far too
//! few words for its length is run again as two halves, keeping whichever
//! reading heard more (an engine that times no words, such as Whisper,
//! could never hear more, so it is not asked). On the microphone track,
//! utterances without a voice are dropped before the engine hears them
//! ([`silent_lines`]). It reports how far through the track it is after
//! each utterance.
//!
//! With a [`PassMemory`], passes over a recording while it is made
//! ([`follow`]) leave the pass after the meeting only the last moments to
//! do: the engine's answers, the voice checks and the echo canceller are
//! remembered by the exact audio they came from.
//!
//! A microphone track with an echo reference (the system track) has the
//! call's echo cancelled from its decoded copy first ([`echo`]), unless
//! the user uses headphones; that pass is the first [`ECHO_SHARE`] of the
//! track's progress.

pub mod echo;
mod echo_resume;
pub mod follow;
mod memory;
mod reading;
pub mod silent_lines;
pub mod utterances;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use deino_core::final_transcription::{
    HeardSpan, TrackError, TrackRequest, TrackTranscriber, fraction_of,
};
use deino_domain::Channel;
use deino_stt::{BatchTranscriber, Millis};

pub use echo::{FixedHeadphones, HeadphonesSetting, cancel_echo};
pub use memory::PassMemory;
use reading::checked;
pub use silent_lines::{DeepFilterCheck, VoiceCheck};
use silent_lines::{drop_silent_lines, voiced};
pub use utterances::{MAX_UTTERANCE, utterances, voice_spans};

use crate::echo::{EchoMode, SonoraAec};

/// Utterances at least this long are checked for a run that stopped early.
pub const CHECKED_UTTERANCE: Millis = Millis(10_000);

/// Fewer words per second than this means the run stopped early.
pub const MIN_WORDS_PER_SECOND: u64 = 1;

/// The share of a microphone track's progress that echo cancellation
/// takes, when it runs; transcribing is the rest.
pub const ECHO_SHARE: f32 = 0.1;

/// Reads a finished track as 16 kHz mono.
pub trait TrackDecoder: Send + Sync {
    /// The track's samples.
    fn decode(&self, path: &Path) -> Result<Vec<f32>, TrackError>;
}

/// Decodes Deino's own files with `ExtAudioFile` (macOS), which trims the
/// AAC priming so sample 0 is the recording's start.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeDecoder;

impl TrackDecoder for NativeDecoder {
    fn decode(&self, path: &Path) -> Result<Vec<f32>, TrackError> {
        deino_audio::decode::read_mono(path, deino_stt::SAMPLE_RATE_HZ)
            .map_err(|error| TrackError::Failed(Box::new(error)))
    }
}

/// Opens the batch engine tracks are transcribed with.
pub trait BatchEngines: Send + Sync {
    /// The engine, loading its model when needed (blocks); unavailable when
    /// no model is downloaded.
    fn open(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError>;

    /// The model [`Self::open`] runs now, naming its answers in a
    /// [`PassMemory`]; `None` (the default) remembers nothing.
    fn model(&self) -> Option<String> {
        None
    }
}

/// The engine behind the `final_transcription` stage.
#[derive(Clone)]
pub struct OfflineTranscriber {
    engines: Arc<dyn BatchEngines>,
    decoder: Arc<dyn TrackDecoder>,
    headphones: Option<Arc<dyn HeadphonesSetting>>,
    voice_check: Option<Arc<dyn VoiceCheck>>,
    memory: Option<PassMemory>,
}

impl OfflineTranscriber {
    /// A transcriber decoding with `decoder` and running `engines`. Without
    /// [`Self::with_headphones`] it treats the output as speakers.
    pub fn new(engines: Arc<dyn BatchEngines>, decoder: Arc<dyn TrackDecoder>) -> Self {
        Self {
            engines,
            decoder,
            headphones: None,
            voice_check: None,
            memory: None,
        }
    }

    /// Shares `memory` with the other passes over the same recording.
    #[must_use]
    pub fn with_memory(self, memory: PassMemory) -> Self {
        Self {
            memory: Some(memory),
            ..self
        }
    }

    /// Drops microphone lines `check` finds no voice under
    /// ([`silent_lines`]); without it every line is kept.
    #[must_use]
    pub fn with_voice_check(self, check: Arc<dyn VoiceCheck>) -> Self {
        Self {
            voice_check: Some(check),
            ..self
        }
    }

    /// Reads "I use headphones" from `setting` before each track: with it
    /// on, no echo is cancelled.
    #[must_use]
    pub fn with_headphones(self, setting: Arc<dyn HeadphonesSetting>) -> Self {
        Self {
            headphones: Some(setting),
            ..self
        }
    }

    /// The reference whose echo to cancel from `request`'s track, if any.
    async fn echo_reference(&self, request: &TrackRequest) -> Option<PathBuf> {
        let reference = request.echo_reference.clone()?;
        let headphones = match &self.headphones {
            Some(setting) => setting.uses_headphones().await,
            None => false,
        };
        (EchoMode::choose(headphones, None) == EchoMode::Cancel).then_some(reference)
    }

    /// Transcribes one track, blocking; checks for cancellation between
    /// utterances and reports progress after each one, as the end of the
    /// audio done over the track's length, then 1.0 once it is finished.
    fn run(
        &self,
        request: &TrackRequest,
        echo: Option<&Path>,
    ) -> Result<Vec<HeardSpan>, TrackError> {
        let (pcm, share) = self.samples(request, echo)?;
        let report = |fraction: f32| request.progress.report(share + (1.0 - share) * fraction);
        let length = Millis::from_samples(pcm.len());
        let check = self.voice_check(request.channel);
        let pieces = utterances(&voice_spans(&pcm), length);
        let pieces = match &check {
            Some(check) => voiced(&pcm, pieces, *check),
            None => pieces,
        };
        if pieces.is_empty() {
            report(1.0);
            return Ok(Vec::new());
        }
        let engine = self.engine()?;
        let mut heard = Vec::with_capacity(pieces.len());
        let mut timed = None;
        for piece in pieces {
            if request.cancel.is_cancelled() {
                return Err(TrackError::Cancelled);
            }
            let done = piece.end;
            heard.extend(checked(engine.as_ref(), &pcm, piece, &mut timed)?);
            report(fraction_of(done.0, length.0));
        }
        report(1.0);
        Ok(match &check {
            Some(check) => drop_silent_lines(&pcm, heard, *check),
            None => heard,
        })
    }

    /// The engine, answering from the memory when there is one.
    fn engine(&self) -> Result<Arc<dyn BatchTranscriber>, TrackError> {
        let engine = self.engines.open()?;
        Ok(match (&self.memory, self.engines.model()) {
            (Some(memory), Some(model)) => memory.engine(engine, model),
            _ => engine,
        })
    }

    /// The voice check of a microphone track. Passes during the meeting
    /// skip it: it is not remembered (see [`PassMemory`]), so only the pass
    /// after the meeting's answers count.
    fn voice_check(&self, channel: Channel) -> Option<&dyn VoiceCheck> {
        let following = self.memory.as_ref().is_some_and(PassMemory::follows);
        self.voice_check
            .as_deref()
            .filter(|_| channel == Channel::Mic && !following)
    }

    /// The track's samples, with `echo`'s echo cancelled when it is given
    /// and readable, and the share of the track's progress that took.
    fn samples(
        &self,
        request: &TrackRequest,
        echo: Option<&Path>,
    ) -> Result<(Vec<f32>, f32), TrackError> {
        let mut pcm = self.decoder.decode(&request.audio)?;
        let Some(reference) = echo else {
            return Ok((pcm, 0.0));
        };
        let far = match self.decoder.decode(reference) {
            Ok(far) => far,
            Err(error) => {
                log::warn!(target: "pipeline", "the system track could not be read, so the microphone is transcribed without echo cancellation: {error}");
                return Ok((pcm, 0.0));
            }
        };
        let progress = |fraction: f32| {
            request.progress.report(fraction * ECHO_SHARE);
            !request.cancel.is_cancelled()
        };
        let finished = match &self.memory {
            Some(memory) => echo_resume::cancel_resumed(memory, (&mut pcm, &far), &progress),
            None => cancel_echo(&mut pcm, &far, &mut SonoraAec::new(), &progress),
        };
        if !finished {
            return Err(TrackError::Cancelled);
        }
        Ok((pcm, ECHO_SHARE))
    }
}

#[async_trait]
impl TrackTranscriber for OfflineTranscriber {
    async fn transcribe(&self, request: TrackRequest) -> Result<Vec<HeardSpan>, TrackError> {
        let echo = self.echo_reference(&request).await;
        // After the meeting, wait for a pass during it to finish first.
        let _gate = match &self.memory {
            Some(memory) if !memory.follows() => Some(memory.shared().await),
            _ => None,
        };
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.run(&request, echo.as_deref()))
            .await
            .map_err(|error| TrackError::Failed(Box::new(error)))?
    }
}

#[cfg(test)]
#[path = "offline_tests.rs"]
mod tests;
