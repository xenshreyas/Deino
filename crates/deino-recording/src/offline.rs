//! Transcribing a finished track after the meeting (CAP-08): the
//! `final_transcription` stage's engine.
//!
//! [`OfflineTranscriber`] decodes the track to 16 kHz ([`TrackDecoder`]),
//! finds its utterances with the lanes' voice detector ([`utterances`]), and
//! runs the `deino-stt` batch engine ([`BatchEngines`]) on each, with word
//! times placed on the recording's timeline. Batch decoding can stop early on
//! some clips (the STT spike), so an utterance that comes back with far too
//! few words for its length is run again as two halves, keeping whichever
//! reading heard more. It reports how far through the track it is after
//! each utterance.

pub mod utterances;

use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use deino_core::final_transcription::{
    HeardSpan, TrackError, TrackRequest, TrackTranscriber, fraction_of,
};
use deino_core::segments::WordTiming;
use deino_stt::words::{TimedWord, words_from_tokens};
use deino_stt::{BatchTranscriber, Millis};

pub use utterances::{MAX_UTTERANCE, utterances, voice_spans};

/// Utterances at least this long are checked for a run that stopped early.
pub const CHECKED_UTTERANCE: Millis = Millis(10_000);

/// Fewer words per second than this means the run stopped early.
pub const MIN_WORDS_PER_SECOND: u64 = 1;

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
}

/// The engine behind the `final_transcription` stage.
#[derive(Clone)]
pub struct OfflineTranscriber {
    engines: Arc<dyn BatchEngines>,
    decoder: Arc<dyn TrackDecoder>,
}

impl OfflineTranscriber {
    /// A transcriber decoding with `decoder` and running `engines`.
    pub fn new(engines: Arc<dyn BatchEngines>, decoder: Arc<dyn TrackDecoder>) -> Self {
        Self { engines, decoder }
    }

    /// Transcribes one track, blocking; checks for cancellation between
    /// utterances and reports progress after each one, as the end of the
    /// audio done over the track's length, then 1.0 once it is finished.
    fn run(&self, request: &TrackRequest) -> Result<Vec<HeardSpan>, TrackError> {
        let pcm = self.decoder.decode(&request.audio)?;
        let length = Millis::from_samples(pcm.len());
        let pieces = utterances(&voice_spans(&pcm), length);
        if pieces.is_empty() {
            request.progress.report(1.0);
            return Ok(Vec::new());
        }
        let engine = self.engines.open()?;
        let mut heard = Vec::with_capacity(pieces.len());
        for piece in pieces {
            if request.cancel.is_cancelled() {
                return Err(TrackError::Cancelled);
            }
            let done = piece.end;
            heard.extend(checked(engine.as_ref(), &pcm, piece)?);
            request.progress.report(fraction_of(done.0, length.0));
        }
        request.progress.report(1.0);
        Ok(heard)
    }
}

#[async_trait]
impl TrackTranscriber for OfflineTranscriber {
    async fn transcribe(&self, request: TrackRequest) -> Result<Vec<HeardSpan>, TrackError> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.run(&request))
            .await
            .map_err(|error| TrackError::Failed(Box::new(error)))?
    }
}

/// The utterance `piece`, read again as two halves when the first reading
/// heard too few words for its length.
fn checked(
    engine: &dyn BatchTranscriber,
    pcm: &[f32],
    piece: Range<Millis>,
) -> Result<Vec<HeardSpan>, TrackError> {
    let whole = heard(engine, pcm, piece.clone())?;
    let length = piece.end.saturating_sub(piece.start);
    let words = whole.as_ref().map_or(0, |span| span.words.len());
    let enough =
        u64::try_from(words).unwrap_or(u64::MAX) >= MIN_WORDS_PER_SECOND * length.0 / 1_000;
    if length < CHECKED_UTTERANCE || enough {
        return Ok(whole.into_iter().collect());
    }
    let middle = Millis(piece.start.0 + length.0 / 2);
    let halves: Vec<HeardSpan> = [piece.start..middle, middle..piece.end]
        .into_iter()
        .map(|half| heard(engine, pcm, half))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();
    let half_words: usize = halves.iter().map(|span| span.words.len()).sum();
    Ok(if half_words > words {
        halves
    } else {
        whole.into_iter().collect()
    })
}

/// What the engine hears in `piece` of `pcm`, if anything.
fn heard(
    engine: &dyn BatchTranscriber,
    pcm: &[f32],
    piece: Range<Millis>,
) -> Result<Option<HeardSpan>, TrackError> {
    let samples = pcm.get(piece.start.to_samples()..piece.end.to_samples().min(pcm.len()));
    let transcript = engine
        .transcribe(samples.unwrap_or_default().to_vec())
        .map_err(|error| TrackError::Failed(Box::new(error)))?;
    let text = transcript.text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let words: Vec<WordTiming> = words_from_tokens(&transcript.tokens, piece.start)
        .iter()
        .map(timing)
        .collect();
    let start = words.first().map_or(stored(piece.start), |word| word.start);
    let end = words.last().map_or(stored(piece.end), |word| word.end);
    Ok(Some(HeardSpan {
        start,
        end: end.max(start),
        text: text.to_owned(),
        words,
    }))
}

fn timing(word: &TimedWord) -> WordTiming {
    WordTiming {
        text: word.text.clone(),
        start: stored(word.start),
        end: stored(word.end.max(word.start)),
    }
}

/// An engine time as a stored time (saturating past 49 days).
fn stored(time: Millis) -> deino_domain::Millis {
    deino_domain::Millis(u32::try_from(time.0).unwrap_or(u32::MAX))
}

#[cfg(test)]
#[path = "offline_tests.rs"]
mod tests;
