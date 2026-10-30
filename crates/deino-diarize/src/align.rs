//! Word timings from wav2vec2 forced alignment: `deino-core`'s
//! `WordAligner` port in Rust (`docs/adr/2026-10-30-speakers-community-1-in-rust.md`).
//!
//! Each line is cut from the track with [`PAD_MS`] either side, run through
//! the bundled wav2vec2-base-960h ([`model`]), and its letters are aligned
//! to the frames ([`ctc`]); a word runs from its first letter's frame to
//! the end of its last. It runs on a blocking thread and loads the model per
//! run, so a finished meeting holds no model; lines run on several sessions
//! at once, and a [`LineMemory`] keeps lines a pass during the meeting
//! timed ([`lines`]).

pub mod ctc;
mod lines;
pub mod model;

pub use lines::{ALIGN_SESSIONS, LineMemory};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use deino_core::diarization::{AlignLine, TrackError, WordAligner};
use deino_core::segments::WordTiming;
use deino_domain::Millis;

use crate::config::RuntimeConfig;
use crate::error::DiarizeError;
use crate::units::SAMPLE_RATE;
use model::{Aligner, CLASSES};

/// Audio kept either side of a line, so its first and last words are whole.
pub const PAD_MS: u32 = 300;

/// The shortest stretch the model can run on (25 ms).
const MIN_SAMPLES: usize = 400;

/// The aligner over the model file at `model`.
pub struct OnnxWordAligner {
    model: PathBuf,
    runtime: RuntimeConfig,
    memory: Option<Arc<LineMemory>>,
    one_at_a_time: tokio::sync::Mutex<()>,
}

impl OnnxWordAligner {
    /// An aligner running the wav2vec2 model at `model`.
    pub fn new(model: PathBuf, runtime: RuntimeConfig) -> Self {
        Self {
            model,
            runtime,
            memory: None,
            one_at_a_time: tokio::sync::Mutex::new(()),
        }
    }

    /// Reuses the lines `memory` holds and keeps the ones it times there.
    #[must_use]
    pub fn with_memory(mut self, memory: Arc<LineMemory>) -> Self {
        self.memory = Some(memory);
        self
    }
}

#[async_trait]
impl WordAligner for OnnxWordAligner {
    async fn align(
        &self,
        audio: &Path,
        lines: &[AlignLine],
    ) -> Result<Vec<Vec<WordTiming>>, TrackError> {
        let _running = self.one_at_a_time.lock().await;
        let (model, runtime) = (self.model.clone(), self.runtime.clone());
        let (audio, lines, memory) = (audio.to_owned(), lines.to_vec(), self.memory.clone());
        let run = tokio::task::spawn_blocking(move || {
            align_file_with((&model, &runtime), (&audio, memory.as_deref()), &lines)
        });
        run.await
            .map_err(|error| TrackError::Failed(Box::new(error)))?
            .map_err(|error| TrackError::Failed(Box::new(error)))
    }
}

/// Decodes the track at `audio` and times `lines` (blocking).
pub fn align_file(
    model: &Path,
    runtime: &RuntimeConfig,
    audio: &Path,
    lines: &[AlignLine],
) -> Result<Vec<Vec<WordTiming>>, DiarizeError> {
    align_file_with((model, runtime), (audio, None), lines)
}

/// [`align_file`], reusing and keeping lines in `memory`.
pub fn align_file_with(
    (model, runtime): (&Path, &RuntimeConfig),
    (audio, memory): (&Path, Option<&LineMemory>),
    lines: &[AlignLine],
) -> Result<Vec<Vec<WordTiming>>, DiarizeError> {
    let samples = deino_audio::read_mono(audio, SAMPLE_RATE)
        .map_err(|error| DiarizeError::Audio(Box::new(error)))?;
    let timed = lines::time_lines((model, runtime), &samples, lines, memory)?;
    log::info!(target: "deino_diarize", "timed the words of {} lines", timed.len());
    Ok(timed)
}

/// One line's words, timed (every word gets a time; see [`ctc::fill_gaps`]).
pub fn align_line(
    aligner: &mut Aligner,
    samples: &[f32],
    line: &AlignLine,
) -> Result<Vec<WordTiming>, DiarizeError> {
    let words: Vec<&str> = line.text.split_whitespace().collect();
    let total_ms = samples.len() * 1000 / SAMPLE_RATE as usize;
    let start = line.start.0.saturating_sub(PAD_MS) as usize;
    let end = (line.end.0.saturating_add(PAD_MS) as usize).min(total_ms);
    let chunk = samples
        .get(start * SAMPLE_RATE as usize / 1000..end * SAMPLE_RATE as usize / 1000)
        .unwrap_or_default();
    let spelled = ctc::tokens(&words);
    let mut times = vec![None; words.len()];
    if !spelled.ids.is_empty() && chunk.len() > MIN_SAMPLES {
        let emission = aligner.emission(chunk)?;
        place(&mut times, &emission, &spelled, (start, end));
    }
    let edges = (f64::from(line.start.0), f64::from(line.end.0));
    ctc::fill_gaps(&mut times, edges);
    Ok(words
        .iter()
        .zip(times)
        .map(|(word, time)| timing(word, time.unwrap_or(edges)))
        .collect())
}

/// Times the words the alignment places, in milliseconds of the recording;
/// none when the chunk has fewer frames than the line has letters.
fn place(
    times: &mut [Option<(f64, f64)>],
    emission: &[f32],
    spelled: &ctc::Tokens,
    (start, end): (usize, usize),
) {
    let frames = emission.len() / CLASSES;
    if frames < spelled.ids.len() {
        return;
    }
    let at = ctc::token_frames(emission, CLASSES, &spelled.ids);
    let per_frame = float(end - start) / float(frames);
    let spans = ctc::word_frames(&at, &spelled.owner, times.len());
    for (time, span) in times.iter_mut().zip(spans) {
        *time = span.map(|(first, last)| {
            (
                float(start) + float(first) * per_frame,
                float(start) + float(last + 1) * per_frame,
            )
        });
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "frame counts and milliseconds are far below 2^52"
)]
fn float(value: usize) -> f64 {
    value as f64
}

/// A word and its span, rounded to whole milliseconds as Python's `round`
/// does (half to even).
fn timing(word: &str, (start, end): (f64, f64)) -> WordTiming {
    let start = to_millis(start);
    WordTiming {
        text: (*word).to_owned(),
        start: Millis(start),
        end: Millis(to_millis(end).max(start)),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "times are non-negative and within the recording"
)]
fn to_millis(ms: f64) -> u32 {
    ms.round_ties_even().max(0.0) as u32
}
