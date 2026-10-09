//! `AVAssetWriter` in CMAF segment-output mode: Deino's live track encoder.
//!
//! `AVFoundation` encodes AAC-LC (64 kbps, 48 kHz, mono) and, instead of
//! writing a file, hands Deino an init segment (`ftyp` + `moov`) and then one
//! `moof` + `mdat` media segment per second through a delegate. Deino appends
//! those bytes to its own `.m4a.part` file, so it decides when bytes hit the
//! disk and knows exactly what is durable (ADR 0014, measured in
//! `docs/references/spike-native-recording.md`: `kill -9` loses 0.03–0.84 s,
//! about 1.7 % of one core per track, no FFmpeg).

mod delegate;
mod finish;
mod pcm;

use std::time::Duration;

use cidre::{arc, av, cm, ut};

use self::delegate::SegmentDelegate;
use self::pcm::{PcmBuffers, aac_input};
use super::{EncodeError, EncoderFactory, SegmentOutput, TrackEncoder};
use crate::units::{Frames, Track};

/// Media segment cadence: bounds `kill -9` loss to about one segment.
pub const SEGMENT_INTERVAL: Duration = Duration::from_secs(1);

/// AAC-LC bit rate: about 29 MB per hour, transparent for speech and STT.
pub const BIT_RATE: u32 = 64_000;

/// Opens [`CmafEncoder`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct CmafEncoderFactory;

impl EncoderFactory for CmafEncoderFactory {
    fn open(
        &self,
        _track: Track,
        output: SegmentOutput,
    ) -> Result<Box<dyn TrackEncoder>, EncodeError> {
        Ok(Box::new(CmafEncoder::start(output)?))
    }
}

/// One track's `AVAssetWriter`, its AAC input and its segment delegate.
struct CmafEncoder {
    writer: arc::R<av::AssetWriter>,
    input: arc::R<av::AssetWriterInput>,
    /// Kept alive for as long as the writer may call it.
    _delegate: arc::R<SegmentDelegate>,
    buffers: PcmBuffers,
}

impl CmafEncoder {
    /// Configures the writer for 1 s CMAF segments and starts the session at
    /// timeline frame zero.
    fn start(output: SegmentOutput) -> Result<Self, EncodeError> {
        let delegate = SegmentDelegate::with(output);
        let mut writer = av::AssetWriter::with_content_type(ut::Type::mpeg4movie())
            .map_err(|_| EncodeError::Rejected("AVAssetWriter initWithContentType:"))?;
        writer.set_output_file_type_profile(Some(av::FileTypeProfile::mpeg4_cmaf_compliant()));
        writer.set_preferred_output_segment_interval(segment_interval());
        writer.set_initial_segment_start_time(cm::Time::zero());
        writer.set_delegate(Some(delegate.as_ref()));
        let input = aac_input()?;
        writer
            .add_input(&input)
            .map_err(|_| EncodeError::Rejected("AVAssetWriter addInput:"))?;
        if !writer.start_writing() {
            return Err(writer_error(&writer, "AVAssetWriter startWriting"));
        }
        writer.start_session_at_src_time(cm::Time::zero());
        Ok(Self {
            writer,
            input,
            _delegate: delegate,
            buffers: PcmBuffers::new()?,
        })
    }
}

impl TrackEncoder for CmafEncoder {
    fn is_ready(&self) -> bool {
        self.input.is_ready_for_more_media_data()
    }

    fn append(&mut self, samples: &[f32], at: Frames) -> Result<(), EncodeError> {
        let buffer = self.buffers.sample_buf(samples, at)?;
        match self.input.append_sample_buf(&buffer) {
            Ok(true) => Ok(()),
            _ => Err(writer_error(
                &self.writer,
                "AVAssetWriterInput appendSampleBuffer:",
            )),
        }
    }

    fn finish(mut self: Box<Self>) -> Result<(), EncodeError> {
        self.input.mark_as_finished();
        finish::finish_writing(&self.writer)?;
        match self.writer.error() {
            Some(_) => Err(writer_error(&self.writer, "AVAssetWriter finishWriting")),
            None => Ok(()),
        }
    }
}

/// [`SEGMENT_INTERVAL`] as a Core Media time.
fn segment_interval() -> cm::Time {
    let seconds = i64::try_from(SEGMENT_INTERVAL.as_secs()).unwrap_or(1);
    cm::Time::new(seconds, 1)
}

/// The writer's error for `call`, or a generic rejection when it has none.
fn writer_error(writer: &av::AssetWriter, call: &'static str) -> EncodeError {
    writer
        .error()
        .map_or(EncodeError::Rejected(call), |error| EncodeError::Writer {
            call,
            code: error.code(),
        })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::fmp4::{AAC_PRIMING, media_packets};
    use crate::sink::MemorySink;

    /// Encodes `seconds` of a 440 Hz tone as fast as the encoder accepts it.
    fn encode(seconds: u64) -> (Arc<MemorySink>, SegmentOutput) {
        let sink = Arc::new(MemorySink::default());
        let output = SegmentOutput {
            sink: sink.clone(),
            ledger: Arc::default(),
        };
        let mut encoder = CmafEncoderFactory
            .open(Track::Microphone, output.clone())
            .unwrap();
        let block: Vec<f32> = (0..480_i16)
            .map(|i| (f32::from(i) * 0.0576).sin() * 0.3)
            .collect();
        let mut at = Frames::ZERO;
        while at < Frames(seconds * 48_000) {
            while !encoder.is_ready() {
                std::thread::yield_now();
            }
            encoder.append(&block, at).unwrap();
            at += Frames(480);
        }
        encoder.finish().unwrap();
        (sink, output)
    }

    #[test]
    fn the_encoder_emits_an_init_segment_then_about_one_segment_per_second() {
        let (sink, output) = encode(3);
        let segments = sink.segments();

        assert_eq!(
            media_packets(&segments[0]),
            0,
            "the first segment is the init segment"
        );
        assert!(
            (4..=5).contains(&segments.len()),
            "{} segments",
            segments.len()
        );
        let encoded = output.ledger.encoded_frames();
        let playable = encoded - AAC_PRIMING;
        assert!(playable >= Frames(3 * 48_000), "{playable:?} playable");
        assert!(
            playable < Frames(3 * 48_000 + 1024),
            "{playable:?} playable"
        );
    }
}
