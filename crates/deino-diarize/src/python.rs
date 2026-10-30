//! The owner's opt-in Python speaker engine (experimental).
//!
//! When `<app data>/python-diarization/` holds a virtual environment and
//! `speakers.py` (`scripts/setup-python-speakers.sh` installs both), Deino
//! diarizes with pyannote Community-1 and times words with a wav2vec2
//! aligner through it, instead of [`crate::track::OnnxTrackDiarizer`].
//! Community-1 embeds voices with the same WeSpeaker ResNet34-LM model, so
//! its centroids carry [`EMBEDDING_MODEL_ID`] and saved voice profiles keep
//! matching. Each call writes the track as raw 16 kHz float32 into the
//! folder's `work/`, runs the script, and reads its JSON answer. The Rust
//! port is planned in `docs/plans/python-speakers-port.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use deino_core::diarization::{
    AlignLine, SpeakerHint, TrackDiarization, TrackDiarizer, TrackError, TrackRequest, Voice,
    VoiceTurn, WordAligner,
};
use deino_core::segments::WordTiming;
use deino_domain::Millis;
use serde::Deserialize;
use tokio::process::Command;

use crate::models::EMBEDDING_MODEL_ID;

/// Sample rate the script reads.
const SAMPLE_RATE: u32 = 16_000;

/// The Python engine under one folder.
#[derive(Debug)]
pub struct PythonEngine {
    python: PathBuf,
    script: PathBuf,
    work: PathBuf,
    one_at_a_time: tokio::sync::Mutex<()>,
}

impl PythonEngine {
    /// The engine installed in `root`, or `None` when it is not installed.
    pub fn installed(root: &Path) -> Option<Self> {
        let python = root.join(".venv/bin/python");
        let script = root.join("speakers.py");
        (python.is_file() && script.is_file()).then(|| Self {
            python,
            script,
            work: root.join("work"),
            one_at_a_time: tokio::sync::Mutex::new(()),
        })
    }

    /// Writes `audio` as raw PCM into the work folder; returns its path.
    async fn pcm(&self, audio: &Path) -> Result<PathBuf, TrackError> {
        let audio = audio.to_path_buf();
        let samples =
            tokio::task::spawn_blocking(move || deino_audio::read_mono(&audio, SAMPLE_RATE))
                .await
                .map_err(failed)?
                .map_err(failed)?;
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        tokio::fs::create_dir_all(&self.work)
            .await
            .map_err(failed)?;
        let path = self.work.join("track.f32");
        tokio::fs::write(&path, bytes).await.map_err(failed)?;
        Ok(path)
    }

    /// Runs the script with `args`; fails with the end of its error output.
    async fn run(&self, args: &[String]) -> Result<(), TrackError> {
        let output = Command::new(&self.python)
            .arg("-I")
            .arg(&self.script)
            .args(args)
            .kill_on_drop(true)
            .output()
            .await
            .map_err(failed)?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr.lines().rev().take(3).collect::<Vec<_>>().join(" | ");
        Err(failed(format!("the Python speaker engine failed: {tail}")))
    }

    async fn answer<T: for<'de> Deserialize<'de>>(&self, file: &Path) -> Result<T, TrackError> {
        let bytes = tokio::fs::read(file).await.map_err(failed)?;
        serde_json::from_slice(&bytes).map_err(failed)
    }
}

#[async_trait]
impl TrackDiarizer for PythonEngine {
    fn embedding_model(&self) -> &str {
        EMBEDDING_MODEL_ID
    }

    async fn diarize(&self, request: TrackRequest) -> Result<TrackDiarization, TrackError> {
        let _running = self.one_at_a_time.lock().await;
        let pcm = self.pcm(&request.audio).await?;
        let out = self.work.join("diarized.json");
        let mut args = vec![
            "diarize".to_owned(),
            "--pcm".to_owned(),
            path_arg(&pcm),
            "--out".to_owned(),
            path_arg(&out),
        ];
        args.extend(bound_args(request.speakers));
        self.run(&args).await?;
        let answer: Diarized = self.answer(&out).await?;
        log::info!(target: "deino_diarize", "python engine: {} voices, {} turns", answer.voices.len(), answer.turns.len());
        Ok(answer.into_diarization())
    }
}

#[async_trait]
impl WordAligner for PythonEngine {
    async fn align(
        &self,
        audio: &Path,
        lines: &[AlignLine],
    ) -> Result<Vec<Vec<WordTiming>>, TrackError> {
        let _running = self.one_at_a_time.lock().await;
        let pcm = self.pcm(audio).await?;
        let asked: Vec<LineIn<'_>> = lines.iter().map(LineIn::from).collect();
        let lines_file = self.work.join("lines.json");
        let json = serde_json::to_vec(&asked).map_err(failed)?;
        tokio::fs::write(&lines_file, json).await.map_err(failed)?;
        let out = self.work.join("aligned.json");
        let args = [
            "align",
            "--pcm",
            &path_arg(&pcm),
            "--lines",
            &path_arg(&lines_file),
            "--out",
            &path_arg(&out),
        ]
        .map(str::to_owned);
        self.run(&args).await?;
        let answer: Aligned = self.answer(&out).await?;
        Ok(answer
            .lines
            .into_iter()
            .map(|words| words.into_iter().map(WordOut::timing).collect())
            .collect())
    }
}

/// `--min-speakers`/`--max-speakers` for the calendar's hint.
fn bound_args(hint: SpeakerHint) -> Vec<String> {
    match hint {
        SpeakerHint::Unknown => Vec::new(),
        SpeakerHint::AtMost(n) => vec!["--max-speakers".to_owned(), n.max(1).to_string()],
        SpeakerHint::Exactly(n) => vec![
            "--min-speakers".to_owned(),
            n.max(1).to_string(),
            "--max-speakers".to_owned(),
            n.max(1).to_string(),
        ],
    }
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn failed(error: impl std::fmt::Display) -> TrackError {
    TrackError::Failed(error.to_string().into())
}

#[derive(serde::Serialize)]
struct LineIn<'a> {
    start_ms: u32,
    end_ms: u32,
    text: &'a str,
}

impl<'a> From<&'a AlignLine> for LineIn<'a> {
    fn from(line: &'a AlignLine) -> Self {
        Self {
            start_ms: line.start.0,
            end_ms: line.end.0,
            text: &line.text,
        }
    }
}

#[derive(Deserialize)]
struct Aligned {
    lines: Vec<Vec<WordOut>>,
}

#[derive(Deserialize)]
struct WordOut {
    text: String,
    start_ms: u32,
    end_ms: u32,
}

impl WordOut {
    fn timing(self) -> WordTiming {
        WordTiming {
            text: self.text,
            start: Millis(self.start_ms),
            end: Millis(self.end_ms.max(self.start_ms)),
        }
    }
}

#[derive(Deserialize)]
struct Diarized {
    turns: Vec<TurnOut>,
    voices: HashMap<String, Vec<f32>>,
}

#[derive(Deserialize)]
struct TurnOut {
    start_ms: u32,
    end_ms: u32,
    speaker: String,
}

impl Diarized {
    /// Voices in order of first appearance, turns pointing at them.
    fn into_diarization(mut self) -> TrackDiarization {
        self.turns.sort_by_key(|turn| turn.start_ms);
        let mut order: Vec<String> = Vec::new();
        let mut result = TrackDiarization::default();
        for turn in self.turns.iter().filter(|t| t.end_ms > t.start_ms) {
            let Some(centroid) = self.voices.get(&turn.speaker) else {
                continue;
            };
            let voice = order
                .iter()
                .position(|label| label == &turn.speaker)
                .unwrap_or_else(|| {
                    order.push(turn.speaker.clone());
                    result.voices.push(Voice {
                        centroid: centroid.clone(),
                        speech: Millis(0),
                    });
                    order.len() - 1
                });
            if let Some(entry) = result.voices.get_mut(voice) {
                entry.speech = Millis(entry.speech.0 + turn.end_ms - turn.start_ms);
            }
            result.turns.push(VoiceTurn {
                start: Millis(turn.start_ms),
                end: Millis(turn.end_ms),
                voice,
            });
        }
        result
    }
}

#[cfg(test)]
#[path = "python_tests.rs"]
mod tests;
