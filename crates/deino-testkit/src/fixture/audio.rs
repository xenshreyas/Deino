//! Fixture audio: the recorded `mic.m4a`/`system.m4a` tracks decoded into
//! PCM that a mock capture source can play back.
//!
//! The tracks are mono AAC, 16 kHz, made by `fixtures/tools/synthesize-audio.mjs`
//! so that each segment is spoken inside its own `[startMs, endMs)` slot on
//! its own channel. FND-05's `MockAudioSource` (selected by `DEINO_MOCK_AUDIO`
//! in debug builds) feeds [`PcmTrack::frames`] into the capture pipeline the
//! way anarlog's `audio-mock` crate does.
//!
//! Symphonia ignores the AAC encoder priming (about 44 ms at 16 kHz is 21 ms),
//! so decoded audio runs a few milliseconds late; fixture checks allow for it.

use std::fs::File;
use std::path::{Path, PathBuf};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::vocab::Millis;

/// Why a fixture track could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// The file could not be opened.
    #[error("could not open {}", path.display())]
    Open {
        /// The audio file.
        path: PathBuf,
        /// The I/O failure.
        source: std::io::Error,
    },
    /// The file is not audio Symphonia can decode.
    #[error("could not decode {}", path.display())]
    Decode {
        /// The audio file.
        path: PathBuf,
        /// Symphonia's error.
        source: SymphoniaError,
    },
}

/// A decoded mono track.
#[derive(Clone, Debug, PartialEq)]
pub struct PcmTrack {
    /// Samples per second.
    pub sample_rate: u32,
    /// Mono samples in `[-1, 1]` (multi-channel files are averaged).
    pub samples: Vec<f32>,
}

impl PcmTrack {
    /// Decodes an audio file into mono PCM at its own sample rate.
    pub fn decode(path: &Path) -> Result<Self, AudioError> {
        let decode_error = |source| AudioError::Decode {
            path: path.to_path_buf(),
            source,
        };
        let file = File::open(path).map_err(|source| AudioError::Open {
            path: path.to_path_buf(),
            source,
        })?;
        let mut track = OpenTrack::open(file).map_err(decode_error)?;
        let sample_rate = track.decoder.codec_params().sample_rate.unwrap_or_default();
        let samples = track.decode_all().map_err(decode_error)?;
        Ok(Self {
            sample_rate,
            samples,
        })
    }

    /// How long the track plays.
    pub fn duration(&self) -> Millis {
        let samples = u64::try_from(self.samples.len()).unwrap_or(u64::MAX);
        Millis(samples.saturating_mul(1000) / u64::from(self.sample_rate.max(1)))
    }

    /// Consecutive frames of `frame_len` samples (the last may be shorter),
    /// the way a capture callback delivers audio.
    pub fn frames(&self, frame_len: usize) -> impl Iterator<Item = &[f32]> {
        self.samples.chunks(frame_len.max(1))
    }

    /// The root-mean-square level of `[from, to)`, or 0 for an empty span.
    pub fn rms(&self, from: Millis, to: Millis) -> f32 {
        let start = self.sample_index(from).unwrap_or(self.samples.len());
        let end = self.sample_index(to).unwrap_or(self.samples.len());
        let span = self.samples.get(start..end).unwrap_or_default();
        if span.is_empty() {
            return 0.0;
        }
        let energy: f32 = span.iter().map(|sample| sample * sample).sum();
        #[allow(
            clippy::cast_precision_loss,
            reason = "a span of fixture audio has far fewer than 2^24 samples"
        )]
        let count = span.len() as f32;
        (energy / count).sqrt()
    }

    /// The index of the sample at `offset`, if the track is that long.
    fn sample_index(&self, offset: Millis) -> Option<usize> {
        let index = offset.0.checked_mul(u64::from(self.sample_rate))? / 1000;
        usize::try_from(index)
            .ok()
            .filter(|index| *index <= self.samples.len())
    }
}

/// A container and a decoder for its default audio track.
struct OpenTrack {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
}

impl OpenTrack {
    /// Probes the container and builds a decoder for its default track.
    fn open(file: File) -> Result<Self, SymphoniaError> {
        let stream = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
        let probed = symphonia::default::get_probe().format(
            &Hint::new(),
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;
        let reader = probed.format;
        let track = reader
            .default_track()
            .ok_or(SymphoniaError::Unsupported("a file without an audio track"))?;
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;
        let track_id = track.id;
        Ok(Self {
            reader,
            decoder,
            track_id,
        })
    }

    /// Decodes every packet of the track into mono samples.
    fn decode_all(&mut self) -> Result<Vec<f32>, SymphoniaError> {
        let mut samples = Vec::new();
        loop {
            let packet = match self.reader.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(error))
                    if error.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(samples);
                }
                Err(error) => return Err(error),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let block = self.decoder.decode(&packet)?;
            let spec = *block.spec();
            let mut buffer = SampleBuffer::<f32>::new(block.capacity() as u64, spec);
            buffer.copy_interleaved_ref(block);
            let channels = spec.channels.count().max(1);
            samples.extend(buffer.samples().chunks(channels).map(average));
        }
    }
}

/// The mean of one interleaved frame.
fn average(frame: &[f32]) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a frame has a handful of channels"
    )]
    let channels = frame.len() as f32;
    frame.iter().sum::<f32>() / channels
}
