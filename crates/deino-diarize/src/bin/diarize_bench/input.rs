//! An audio file and its RTTM ground truth (the spike's synthetic clips,
//! or the `AMI` and `VoxConverse` references).

use std::path::Path;

use deino_diarize::metrics::ReferenceTurn;
use deino_diarize::policy::ExpectedSpeakers;
use deino_domain::Millis;

use crate::{Case, RATE};

/// A case from `audio` and `rttm`; `speakers` is the exact count the
/// calendar would give, if any.
pub(crate) fn case(audio: &str, rttm: &str, speakers: Option<usize>) -> Result<Case, String> {
    let name = Path::new(audio).file_stem().map_or_else(
        || audio.to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    Ok(Case {
        name,
        samples: samples(Path::new(audio))?,
        reference: reference(&std::fs::read_to_string(rttm).map_err(|e| format!("{rttm}: {e}"))?)?,
        expected: speakers.map_or(ExpectedSpeakers::Unknown, ExpectedSpeakers::Exactly),
    })
}

/// 16 kHz mono samples: WAV through hound, anything else through
/// `ExtAudioFile` (macOS).
fn samples(path: &Path) -> Result<Vec<f32>, String> {
    if path.extension().is_none_or(|extension| extension != "wav") {
        return deino_audio::read_mono(path, RATE).map_err(|e| format!("{}: {e}", path.display()));
    }
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    if spec.sample_rate != RATE || spec.channels != 1 || spec.bits_per_sample != 16 {
        return Err(format!(
            "{}: expected 16 kHz mono 16-bit PCM",
            path.display()
        ));
    }
    reader
        .samples::<i16>()
        .map(|sample| {
            sample
                .map(|value| f32::from(value) / 32_768.0)
                .map_err(|e| e.to_string())
        })
        .collect()
}

/// `SPEAKER <file> <channel> <start> <duration> <NA> <NA> <speaker> ...`
/// lines; anything else is skipped.
fn reference(rttm: &str) -> Result<Vec<ReferenceTurn>, String> {
    rttm.lines()
        .map(str::split_whitespace)
        .filter_map(|mut fields| (fields.next() == Some("SPEAKER")).then_some(fields))
        .map(|fields| {
            let fields: Vec<&str> = fields.collect();
            let (start, duration, speaker) = match fields.as_slice() {
                [_, _, start, duration, _, _, speaker, ..] => {
                    (millis(start)?, millis(duration)?, speaker)
                }
                _ => return Err("a malformed RTTM line".to_owned()),
            };
            Ok(ReferenceTurn {
                start,
                end: Millis(start.get() + duration.get()),
                speaker: (*speaker).to_owned(),
            })
        })
        .collect()
}

fn millis(seconds: &str) -> Result<Millis, String> {
    let value: f64 = seconds
        .parse()
        .map_err(|_| format!("{seconds} is not a time"))?;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "RTTM times are non-negative seconds"
    )]
    let millis = (value * 1_000.0).round().max(0.0) as u32;
    Ok(Millis(millis))
}
