//! [`transcode`]: the first decoder that reads the whole file wins, and its
//! audio is written as 48 kHz mono AAC.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{MediaDecoder, MediaError, PcmStream};
use crate::units::Frames;

/// What [`transcode`] wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transcoded {
    /// The audio's length at 48 kHz.
    pub length: Frames,
    /// The decoder that read the file.
    pub decoder: &'static str,
}

/// Converts the audio of `source` into a 48 kHz mono AAC `.m4a` at
/// `target`, trying `decoders` in order until one reads the whole file.
///
/// Returns the first decoder's error when every decoder fails (it is the
/// most specific: symphonia knows the most containers), and removes any
/// partial `target`. `cancel` is checked between blocks.
pub fn transcode(
    decoders: &[&dyn MediaDecoder],
    source: &Path,
    target: &Path,
    cancel: &AtomicBool,
) -> Result<Transcoded, MediaError> {
    let mut first_error = None;
    for decoder in decoders {
        match convert(*decoder, source, target, cancel) {
            Ok(length) => {
                let decoder = decoder.name();
                return Ok(Transcoded { length, decoder });
            }
            Err(MediaError::Cancelled) => {
                first_error = Some(MediaError::Cancelled);
                break;
            }
            Err(error) => {
                log::info!(target: "media", "{} could not read the file: {error:?}", decoder.name());
                first_error.get_or_insert(error);
            }
        }
    }
    remove_partial(target);
    Err(first_error.unwrap_or(MediaError::UnsupportedFormat))
}

/// Reads `stream` to its end into `write`; returns the frames written.
///
/// A stream that ends without a single sample has no audio to import.
pub fn pump(
    stream: &mut dyn PcmStream,
    write: &mut dyn FnMut(&[f32]) -> Result<(), MediaError>,
    cancel: &AtomicBool,
) -> Result<Frames, MediaError> {
    let mut frames = 0_usize;
    while let Some(block) = stream.next_block()? {
        if cancel.load(Ordering::Relaxed) {
            return Err(MediaError::Cancelled);
        }
        write(&block)?;
        frames += block.len();
    }
    if frames == 0 {
        return Err(MediaError::NoAudioTrack);
    }
    Ok(Frames::of_len(frames))
}

/// One decoder's attempt: decode `source` and encode it into `target`.
#[cfg(target_os = "macos")]
fn convert(
    decoder: &dyn MediaDecoder,
    source: &Path,
    target: &Path,
    cancel: &AtomicBool,
) -> Result<Frames, MediaError> {
    use crate::finalize::ext_audio::{AacWriter, Container};

    let mut stream = decoder.open(source)?;
    let mut writer = AacWriter::create(target, Container::M4a).map_err(MediaError::Encode)?;
    let length = pump(
        stream.as_mut(),
        &mut |samples| writer.write(samples).map_err(MediaError::Encode),
        cancel,
    )?;
    writer.close();
    Ok(length)
}

/// Encoding AAC needs `AudioToolbox`.
#[cfg(not(target_os = "macos"))]
fn convert(
    _decoder: &dyn MediaDecoder,
    _source: &Path,
    _target: &Path,
    _cancel: &AtomicBool,
) -> Result<Frames, MediaError> {
    Err(MediaError::Unsupported)
}

/// Deletes what a failed attempt left at `target`.
fn remove_partial(target: &Path) {
    match std::fs::remove_file(target) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => log::warn!(target: "media", "could not remove a partial conversion: {error}"),
    }
}
