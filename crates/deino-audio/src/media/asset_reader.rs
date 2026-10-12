//! [`AssetReaderDecoder`]: `AVURLAsset` + `AVAssetReader` (macOS), the
//! fallback for what symphonia cannot read: Apple's ADTS (`0xFFF9`),
//! AAC-in-CAF and anything else `AVFoundation` plays, video included.
//! `AVFoundation` mixes down and resamples: the output is asked for as
//! 48 kHz mono f32 (the spike's `src/av_reader.rs`).

use std::path::Path;
use std::sync::mpsc;

use cidre::{arc, av, cat, ns};

use super::{MediaDecoder, MediaError, PcmStream};
use crate::units::STORAGE_RATE;

/// Decodes through `AVFoundation`.
#[derive(Clone, Copy, Debug, Default)]
pub struct AssetReaderDecoder;

impl MediaDecoder for AssetReaderDecoder {
    fn name(&self) -> &'static str {
        "AVAssetReader"
    }

    fn open(&self, path: &Path) -> Result<Box<dyn PcmStream>, MediaError> {
        let (asset, track) = first_audio_track(path)?;
        let mut reader = av::AssetReader::with_asset(&asset).map_err(ns_error)?;
        let settings = pcm_settings();
        let mut output = av::AssetReaderTrackOutput::with_track(&track, Some(&settings))
            .map_err(|_| MediaError::UnsupportedFormat)?;
        output.set_always_copies_sample_data(false);
        reader
            .add_output(&output)
            .map_err(|_| MediaError::UnsupportedFormat)?;
        let started = reader
            .start_reading()
            .map_err(|_| MediaError::UnsupportedFormat)?;
        if !started {
            return Err(reader
                .error()
                .map_or(MediaError::UnsupportedFormat, |e| ns_error(e.as_ref())));
        }
        Ok(Box::new(AssetStream { reader, output }))
    }
}

/// One file being read.
struct AssetStream {
    reader: arc::R<av::AssetReader>,
    output: arc::R<av::AssetReaderTrackOutput>,
}

impl PcmStream for AssetStream {
    fn next_block(&mut self) -> Result<Option<Vec<f32>>, MediaError> {
        let Ok(Some(buffer)) = self.output.next_sample_buf() else {
            return match self.reader.error() {
                Some(error) => Err(ns_error(error.as_ref())),
                None => Ok(None),
            };
        };
        let list = buffer
            .audio_buf_list::<1>()
            .map_err(|status| MediaError::decode(format!("audio buffer list: {}", status.0)))?;
        let block = list
            .list()
            .buffers
            .first()
            .map(samples_of)
            .unwrap_or_default();
        Ok(Some(block.to_vec()))
    }
}

/// The f32 samples in one buffer of an `AudioBufferList`.
#[allow(
    unsafe_code,
    clippy::cast_ptr_alignment,
    reason = "AVAssetReader hands its PCM out as a raw AudioBuffer; it lives as long as the sample buffer, and its alignment is checked"
)]
fn samples_of(buffer: &cidre::at::AudioBuf) -> &[f32] {
    let count = usize::try_from(buffer.data_bytes_size).unwrap_or_default() / size_of::<f32>();
    let aligned = buffer.data.align_offset(align_of::<f32>()) == 0;
    if buffer.data.is_null() || count == 0 || !aligned {
        return &[];
    }
    // SAFETY: the reader was asked for packed f32 PCM; `data` points to
    // `data_bytes_size` bytes owned by the sample buffer borrowed above.
    unsafe { std::slice::from_raw_parts(buffer.data.cast::<f32>(), count) }
}

/// The asset's track list, moved out of the completion handler's thread.
struct SendTracks(Option<arc::R<ns::Array<av::asset::Track>>>);

// SAFETY: the array is retained and only read on the receiving thread after
// the handler has returned.
#[allow(unsafe_code, reason = "carries a retained NSArray across one channel")]
unsafe impl Send for SendTracks {}

/// The asset at `path` and its first audio track.
fn first_audio_track(
    path: &Path,
) -> Result<(arc::R<av::UrlAsset>, arc::R<av::asset::Track>), MediaError> {
    use cidre::arc::Retain;

    let url = ns::Url::with_fs_path_str(&path.to_string_lossy(), false);
    let asset = av::UrlAsset::with_url(&url, None).ok_or(MediaError::UnsupportedFormat)?;
    let (sender, receiver) = mpsc::channel();
    asset.load_tracks_with_media_type_block(av::MediaType::audio(), move |tracks, error| {
        let failed = error.is_some();
        // The receiver waits below; it is only gone if this thread panicked.
        let _unheard = sender.send((SendTracks(tracks.map(Retain::retained)), failed));
    });
    let (tracks, failed) = receiver.recv().map_err(|_| MediaError::UnsupportedFormat)?;
    let track = tracks
        .0
        .and_then(|tracks| tracks.get(0).ok().map(|track| track.retained()));
    match (track, failed) {
        (Some(track), _) => Ok((asset, track)),
        (None, true) => Err(MediaError::UnsupportedFormat),
        (None, false) => Err(MediaError::NoAudioTrack),
    }
}

/// 48 kHz mono packed f32: `AVFoundation` downmixes and resamples.
fn pcm_settings() -> arc::R<ns::Dictionary<ns::String, ns::Id>> {
    use av::audio::{all_formats_keys as format, linear_pcm_keys as pcm};
    let rate = ns::Number::with_f64(f64::from(STORAGE_RATE));
    let one = ns::Number::with_i32(1);
    let bits = ns::Number::with_i32(32);
    let yes = ns::Number::with_bool(true);
    let no = ns::Number::with_bool(false);
    ns::Dictionary::<ns::String, ns::Id>::with_keys_values(
        &[
            format::id(),
            format::sample_rate(),
            format::number_of_channels(),
            pcm::bit_depth(),
            pcm::is_float(),
            pcm::is_non_interleaved(),
        ],
        &[
            cat::AudioFormat::LINEAR_PCM.as_ref(),
            rate.as_ref(),
            one.as_ref(),
            bits.as_ref(),
            yes.as_ref(),
            no.as_ref(),
        ],
    )
}

/// An `NSError` as a decode failure, keeping its description for the logs.
fn ns_error(error: &ns::Error) -> MediaError {
    MediaError::decode(error.localized_description().to_string())
}
