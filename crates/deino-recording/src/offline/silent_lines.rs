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
//! judges. When the model fails, every line is kept.

use std::ops::Range;

use deino_core::final_transcription::HeardSpan;
use deino_denoise::{Cleaner, frames_of, peak_db};

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

/// [`VoiceCheck`] with DeepFilterNet3, one model per track.
#[derive(Clone, Copy, Debug, Default)]
pub struct DeepFilterCheck;

impl VoiceCheck for DeepFilterCheck {
    fn cleaned_peaks(&self, pcm: &[f32], frames: &[Range<usize>]) -> Result<Vec<f32>, String> {
        let mut cleaner = Cleaner::new().map_err(|error| error.to_string())?;
        frames
            .iter()
            .map(|span| cleaner.cleaned_peak(pcm, span.clone()))
            .collect::<Result<_, _>>()
            .map_err(|error| error.to_string())
    }
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
