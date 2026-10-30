//! Lines heard on the microphone where nobody spoke (pure rule, model port).
//!
//! Whisper writes text for noise: "Thank you." for a burst of typing,
//! sometimes a whole invented sentence. A microphone line is dropped when
//! two checks agree it holds no voice:
//!
//! 1. the raw (echo-cancelled) microphone stays below [`SPEAKING_DB`] for
//!    the whole line, quieter than the user's own speech, and
//! 2. a noise-removal model ([`VoiceCheck`], DeepFilterNet) cleans it to
//!    below [`NO_VOICE_DB`], near silence.
//!
//! Only lines that pass the first, cheap check go to the model. The audio
//! the engine hears is never cleaned: transcribing cleaned audio cut real
//! speech in the A/B test that chose this rule (Oct 2026); the model only
//! judges. When the model fails, every line is kept. The utterances are
//! checked before the engine hears them ([`voiced`]), so it never spends a
//! run on one without a voice, and the lines again after.

use std::ops::Range;

use deino_core::final_transcription::HeardSpan;
use deino_denoise::{Cleaner, frames_of, peak_db};
use deino_stt::Millis;

/// Louder than this on the raw microphone, a line may be the user speaking
/// (their speech peaked at -18 to -31 dB in the A/B test; noise lines at
/// about -40 dB).
pub const SPEAKING_DB: f32 = -35.0;

/// Cleaned audio quieter than this holds no voice (near silence; the
/// user's quietest real line cleaned to about -61 dB).
pub const NO_VOICE_DB: f32 = -70.0;

/// Measures stretches of a 16 kHz microphone track once noise is removed.
pub trait VoiceCheck: Send + Sync {
    /// For each of `frames` (100 ms frames on the track's grid), the loudest
    /// frame of the cleaned audio, in dBFS.
    fn cleaned_peaks(&self, pcm: &[f32], frames: &[Range<usize>]) -> Result<Vec<f32>, String>;
}

/// Most threads [`DeepFilterCheck`] cleans on at once.
const CHECK_THREADS: usize = 6;

/// [`VoiceCheck`] with DeepFilterNet3: the lines in contiguous shares, one
/// model per share, on up to [`CHECK_THREADS`] threads (half the cores).
#[derive(Clone, Copy, Debug, Default)]
pub struct DeepFilterCheck;

impl VoiceCheck for DeepFilterCheck {
    fn cleaned_peaks(&self, pcm: &[f32], frames: &[Range<usize>]) -> Result<Vec<f32>, String> {
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        let share = frames
            .len()
            .div_ceil((cores / 2).clamp(1, CHECK_THREADS))
            .max(1);
        let parts: Vec<Result<Vec<f32>, String>> = std::thread::scope(|scope| {
            let handles: Vec<_> = frames
                .chunks(share)
                .map(|part| scope.spawn(move || clean_share(pcm, part)))
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .unwrap_or_else(|_| Err("a voice check thread panicked".into()))
                })
                .collect()
        });
        let mut peaks = Vec::with_capacity(frames.len());
        for part in parts {
            peaks.extend(part?);
        }
        Ok(peaks)
    }
}

/// The cleaned peaks of `part`'s lines with one model.
fn clean_share(pcm: &[f32], part: &[Range<usize>]) -> Result<Vec<f32>, String> {
    let mut cleaner = Cleaner::new().map_err(|error| error.to_string())?;
    part.iter()
        .map(|span| cleaner.cleaned_peak(pcm, span.clone()))
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())
}

/// `pieces` of 16 kHz `pcm` without those that hold no voice.
pub fn voiced(
    pcm: &[f32],
    pieces: Vec<Range<Millis>>,
    check: &dyn VoiceCheck,
) -> Vec<Range<Millis>> {
    let as_span = |piece: &Range<Millis>| HeardSpan {
        start: stored(piece.start),
        end: stored(piece.end),
        text: String::new(),
        words: Vec::new(),
    };
    let kept = drop_silent_lines(pcm, pieces.iter().map(as_span).collect(), check);
    let mut kept = kept.iter().map(|span| span.start).peekable();
    pieces
        .into_iter()
        .filter(|piece| kept.next_if_eq(&stored(piece.start)).is_some())
        .collect()
}

fn stored(time: Millis) -> deino_domain::Millis {
    deino_domain::Millis(u32::try_from(time.0).unwrap_or(u32::MAX))
}

/// `heard` without the lines of 16 kHz `pcm` that hold no voice.
pub fn drop_silent_lines(
    pcm: &[f32],
    heard: Vec<HeardSpan>,
    check: &dyn VoiceCheck,
) -> Vec<HeardSpan> {
    let frames: Vec<Range<usize>> = heard
        .iter()
        .map(|span| frames_of(u64::from(span.start.0), u64::from(span.end.0)))
        .collect();
    let quiet: Vec<usize> = (0..heard.len())
        .filter(|&at| {
            frames
                .get(at)
                .is_some_and(|f| peak_db(pcm, f.clone()) < SPEAKING_DB)
        })
        .collect();
    if quiet.is_empty() {
        return heard;
    }
    let asked: Vec<Range<usize>> = quiet
        .iter()
        .filter_map(|&at| frames.get(at).cloned())
        .collect();
    let peaks = match check.cleaned_peaks(pcm, &asked) {
        Ok(peaks) if peaks.len() == asked.len() => peaks,
        Ok(_) | Err(_) => {
            log::warn!(target: "pipeline", "the voice check failed, so every microphone line is kept");
            return heard;
        }
    };
    let silent: Vec<usize> = quiet
        .iter()
        .zip(peaks)
        .filter(|(_, peak)| *peak < NO_VOICE_DB)
        .map(|(&at, _)| at)
        .collect();
    log::info!(target: "pipeline", "voice check: {} of {} microphone lines were quiet, {} had no voice and were dropped", quiet.len(), heard.len(), silent.len());
    heard
        .into_iter()
        .enumerate()
        .filter(|(at, _)| !silent.contains(at))
        .map(|(_, span)| span)
        .collect()
}

#[cfg(test)]
mod silent_lines_tests;
