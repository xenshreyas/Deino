//! PCM → `CMSampleBuffer`, and the AAC writer input that consumes it.
//!
//! Each buffer's presentation timestamp is its timeline frame over
//! 48 000 — the sample index is time — so the encoder never sees host-clock
//! jitter.

use cidre::at::audio;
use cidre::{arc, av, cat, cm, ns};

use super::BIT_RATE;
use crate::encoder::EncodeError;
use crate::units::{Frames, STORAGE_RATE};

/// Bytes per stored sample (f32).
const SAMPLE_BYTES: usize = std::mem::size_of::<f32>();

/// 48 kHz mono f32, the format the writer thread delivers.
fn pcm_asbd() -> audio::StreamBasicDesc {
    audio::StreamBasicDesc::common_f32(f64::from(STORAGE_RATE), 1, true)
}

/// The AAC-LC, 64 kbps, 48 kHz mono writer input, fed f32 PCM in real time.
pub(super) fn aac_input() -> Result<arc::R<av::AssetWriterInput>, EncodeError> {
    use av::audio::all_formats_keys as keys;
    use av::audio::settings::encoder_propery_keys as encoder_keys;
    let rate = ns::Number::with_f64(f64::from(STORAGE_RATE));
    let channels = ns::Number::with_i32(1);
    let bit_rate = ns::Number::with_u32(BIT_RATE);
    let settings = ns::Dictionary::<ns::String, ns::Id>::with_keys_values(
        &[
            keys::id(),
            keys::sample_rate(),
            keys::number_of_channels(),
            encoder_keys::bit_rate(),
        ],
        &[
            cat::AudioFormat::MPEG4_AAC.as_ref(),
            rate.as_ref(),
            channels.as_ref(),
            bit_rate.as_ref(),
        ],
    );
    let hint = cm::AudioFormatDesc::with_asbd(&pcm_asbd()).map_err(|status| EncodeError::Os {
        call: "CMAudioFormatDescriptionCreate",
        status: status.0.get(),
    })?;
    let mut input = av::AssetWriterInput::with_media_type_output_settings_source_format_hint(
        av::MediaType::audio(),
        Some(settings.as_ref()),
        Some(&hint),
    )
    .map_err(|_| EncodeError::Rejected("AVAssetWriterInput init"))?;
    input.set_expects_media_data_in_real_time(true);
    Ok(input)
}

/// Builds f32 mono sample buffers stamped with their timeline position.
pub(super) struct PcmBuffers {
    format: arc::R<cm::AudioFormatDesc>,
}

impl PcmBuffers {
    /// Prepares the 48 kHz mono f32 format description once.
    pub(super) fn new() -> Result<Self, EncodeError> {
        let format =
            cm::AudioFormatDesc::with_asbd(&pcm_asbd()).map_err(|status| EncodeError::Os {
                call: "CMAudioFormatDescriptionCreate",
                status: status.0.get(),
            })?;
        Ok(Self { format })
    }

    /// Copies `samples` into a new sample buffer whose first frame is `at`.
    pub(super) fn sample_buf(
        &self,
        samples: &[f32],
        at: Frames,
    ) -> Result<arc::R<cm::SampleBuf>, EncodeError> {
        let block = copy_into_block(samples)?;
        let rate = i32::try_from(STORAGE_RATE).unwrap_or(i32::MAX);
        let timing = cm::SampleTimingInfo {
            duration: cm::Time::new(1, rate),
            pts: cm::Time::new(i64::try_from(at.0).unwrap_or(i64::MAX), rate),
            dts: cm::Time::invalid(),
        };
        create_sample_buf(&block, &self.format, samples.len(), &timing)
    }
}

/// A Core Media block buffer holding a copy of `samples`.
fn copy_into_block(samples: &[f32]) -> Result<arc::R<cm::BlockBuf>, EncodeError> {
    let os = |status: cidre::os::Error| EncodeError::Os {
        call: "CMBlockBufferCreateWithMemoryBlock",
        status: status.0.get(),
    };
    let mut block = cm::BlockBuf::with_mem_block(samples.len() * SAMPLE_BYTES, None).map_err(os)?;
    let bytes = block.as_mut_slice().map_err(os)?;
    for (slot, sample) in bytes.chunks_exact_mut(SAMPLE_BYTES).zip(samples) {
        slot.copy_from_slice(&sample.to_ne_bytes());
    }
    Ok(block)
}

/// `CMSampleBufferCreate` for `count` uniformly timed f32 samples.
#[allow(
    unsafe_code,
    reason = "CMSampleBufferCreate takes raw pointers to the timing and size arrays"
)]
fn create_sample_buf(
    block: &cm::BlockBuf,
    format: &cm::AudioFormatDesc,
    count: usize,
    timing: &cm::SampleTimingInfo,
) -> Result<arc::R<cm::SampleBuf>, EncodeError> {
    let mut out = None;
    // SAFETY: `timing` and `SAMPLE_BYTES` outlive the call and each describe
    // one entry, matching the counts passed; `out` is a valid out-pointer.
    let created = unsafe {
        cm::SampleBuf::create_in(
            None,
            Some(block),
            true,
            None,
            std::ptr::null(),
            Some(format),
            count.cast_signed(),
            1,
            timing,
            1,
            &SAMPLE_BYTES,
            &raw mut out,
        )
    };
    created.map_err(|status| EncodeError::Os {
        call: "CMSampleBufferCreate",
        status: status.0.get(),
    })?;
    out.ok_or(EncodeError::Rejected("CMSampleBufferCreate"))
}
