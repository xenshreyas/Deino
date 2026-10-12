//! [`SymphoniaDecoder`]: pure-Rust demuxing and decoding, tried first.
//!
//! symphonia's formats and codecs (`all`) plus libopus (BSD-3, through
//! `symphonia-adapter-libopus`) for Opus in Ogg, CAF, MKV and `WebM`. Packets
//! of the first audio track are decoded, mixed down to mono and resampled
//! to 48 kHz once ([`ChunkResampler`]). A few damaged packets are skipped;
//! more than [`MAX_DECODE_ERRORS`] fail the file so the next decoder can
//! try it.
//!
//! symphonia 0.5 ignores AAC encoder priming, so AAC starts about 44 ms
//! late; for an imported file that offset is inaudible and harmless.

use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, CodecRegistry, Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::{MediaDecoder, MediaError, PcmStream};
use crate::capture::resample::ChunkResampler;
use crate::units::SampleRate;

/// Damaged packets skipped before the file counts as unreadable.
pub const MAX_DECODE_ERRORS: u32 = 64;

/// Decodes with symphonia and libopus.
#[derive(Clone)]
pub struct SymphoniaDecoder {
    codecs: Arc<CodecRegistry>,
}

impl std::fmt::Debug for SymphoniaDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymphoniaDecoder").finish_non_exhaustive()
    }
}

impl Default for SymphoniaDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SymphoniaDecoder {
    /// symphonia's codecs with libopus registered for Opus.
    pub fn new() -> Self {
        let mut codecs = CodecRegistry::new();
        symphonia::default::register_enabled_codecs(&mut codecs);
        codecs.register_all::<symphonia_adapter_libopus::OpusDecoder>();
        Self {
            codecs: Arc::new(codecs),
        }
    }

    /// The probed container of `path`.
    fn probe(path: &Path) -> Result<Box<dyn FormatReader>, MediaError> {
        let file = File::open(path)?;
        let source = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
            hint.with_extension(extension);
        }
        let options = FormatOptions {
            enable_gapless: true,
            ..FormatOptions::default()
        };
        symphonia::default::get_probe()
            .format(&hint, source, &options, &MetadataOptions::default())
            .map(|probed| probed.format)
            .map_err(open_error)
    }
}

impl MediaDecoder for SymphoniaDecoder {
    fn name(&self) -> &'static str {
        "symphonia"
    }

    fn open(&self, path: &Path) -> Result<Box<dyn PcmStream>, MediaError> {
        let format = Self::probe(path)?;
        let track = format
            .tracks()
            .iter()
            .find(|track| {
                track.codec_params.codec != CODEC_TYPE_NULL
                    && track.codec_params.sample_rate.is_some()
            })
            .ok_or(MediaError::NoAudioTrack)?;
        let track_id = track.id;
        let decoder = self
            .codecs
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(open_error)?;
        Ok(Box::new(SymphoniaStream {
            format,
            decoder,
            track_id,
            mono: MonoMixer::default(),
            resampler: None,
            errors: 0,
        }))
    }
}

/// One open file.
struct SymphoniaStream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    mono: MonoMixer,
    resampler: Option<ChunkResampler>,
    errors: u32,
}

impl PcmStream for SymphoniaStream {
    fn next_block(&mut self) -> Result<Option<Vec<f32>>, MediaError> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(error)) if error.kind() == ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(error) => return Err(MediaError::decode(error)),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(buffer) => {
                    let rate = buffer.spec().rate;
                    let mono = self.mono.mix(buffer);
                    let block = resample(&mut self.resampler, rate, mono)?;
                    if !block.is_empty() {
                        return Ok(Some(block));
                    }
                }
                Err(SymphoniaError::DecodeError(_)) => self.skip_damaged()?,
                Err(error) => return Err(MediaError::decode(error)),
            }
        }
    }
}

impl SymphoniaStream {
    /// Counts a damaged packet; too many fail the file.
    fn skip_damaged(&mut self) -> Result<(), MediaError> {
        self.errors += 1;
        if self.errors > MAX_DECODE_ERRORS {
            return Err(MediaError::decode("too many damaged packets"));
        }
        Ok(())
    }
}

/// `mono` at 48 kHz; the resampler is built for the first block's rate.
fn resample(
    resampler: &mut Option<ChunkResampler>,
    rate: u32,
    mono: &[f32],
) -> Result<Vec<f32>, MediaError> {
    let resampler = match resampler {
        Some(resampler) => resampler,
        empty => empty.insert(
            ChunkResampler::new(SampleRate(rate), SampleRate::STORAGE)
                .map_err(MediaError::decode)?,
        ),
    };
    let mut out = Vec::with_capacity(mono.len() * 2);
    resampler.process(mono, |block, _| out.extend_from_slice(block));
    Ok(out)
}

/// Mixes decoded buffers down to mono, reusing its buffers.
#[derive(Default)]
struct MonoMixer {
    interleaved: Option<SampleBuffer<f32>>,
    mono: Vec<f32>,
}

impl MonoMixer {
    /// The average of every channel of `buffer`.
    fn mix(&mut self, buffer: symphonia::core::audio::AudioBufferRef<'_>) -> &[f32] {
        let spec = *buffer.spec();
        let frames = u64::try_from(buffer.capacity()).unwrap_or(u64::MAX);
        let samples = match &mut self.interleaved {
            Some(samples) if samples.capacity() >= buffer.capacity() * spec.channels.count() => {
                samples
            }
            slot => slot.insert(SampleBuffer::new(frames, spec)),
        };
        samples.copy_interleaved_ref(buffer);
        let channels = spec.channels.count().max(1);
        // Exact for every real channel count (≤ 32).
        #[allow(clippy::cast_precision_loss, reason = "channel counts are tiny")]
        let scale = 1.0 / channels as f32;
        self.mono.clear();
        self.mono.extend(
            samples
                .samples()
                .chunks_exact(channels)
                .map(|frame| frame.iter().sum::<f32>() * scale),
        );
        &self.mono
    }
}

/// Maps a probe or codec setup error: an unknown container or codec is
/// [`MediaError::UnsupportedFormat`].
fn open_error(error: SymphoniaError) -> MediaError {
    match error {
        SymphoniaError::Unsupported(_) => MediaError::UnsupportedFormat,
        SymphoniaError::IoError(error) if error.kind() == ErrorKind::UnexpectedEof => {
            MediaError::UnsupportedFormat
        }
        SymphoniaError::IoError(error) => MediaError::Io(error),
        other => MediaError::decode(other),
    }
}
