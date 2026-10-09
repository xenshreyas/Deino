//! `ExtAudioFile` streaming reader and AAC writer for finalize.
//!
//! Reading Deino's own AAC files through `ExtAudioFile` (which `AVAudioFile` is
//! built on) honours the edit list, so the 2112 frames of encoder priming
//! are trimmed and frame 0 is the recording epoch. symphonia 0.5 ignores the
//! priming and would shift everything 44 ms late.

use std::ffi::c_void;
use std::path::Path;

use cidre::at::{AudioBuf, AudioBufList, ExtAudioFile, ExtAudioFileRef, audio};
use cidre::cf;

use super::FinalizeError;
use crate::fmp4::AAC_FRAMES_PER_PACKET;
use crate::units::STORAGE_RATE;

/// `kExtAudioFileProperty_ClientDataFormat`.
const CLIENT_DATA_FORMAT: u32 = u32::from_be_bytes(*b"cfmt");
/// `kExtAudioFileProperty_AudioConverter`.
const AUDIO_CONVERTER: u32 = u32::from_be_bytes(*b"acnv");
/// `kExtAudioFileProperty_ConverterConfig`.
const CONVERTER_CONFIG: u32 = u32::from_be_bytes(*b"accf");
/// `kAudioConverterEncodeBitRate`.
const ENCODE_BIT_RATE: u32 = u32::from_be_bytes(*b"brat");

/// Bit rate of the finalized files: the same 64 kbps as the live tracks.
const BIT_RATE: u32 = 64_000;

/// The container a finalized file is written in.
#[derive(Clone, Copy, Debug)]
pub(super) enum Container {
    /// `.m4a` (Apple MPEG-4 audio).
    M4a,
    /// `.mp4` (MPEG-4).
    Mp4,
}

/// Streams any `AudioToolbox`-readable file as mono f32 at a chosen rate.
pub(crate) struct PcmReader {
    file: ExtAudioFileRef,
}

impl PcmReader {
    /// Opens `path` and asks for mono f32 output at `sample_rate` (the
    /// converter resamples when the file's rate differs).
    pub(crate) fn open(path: &Path, sample_rate: u32) -> Result<Self, FinalizeError> {
        let url = file_url(path)?;
        let mut file = ExtAudioFile::open(&url).map_err(os("ExtAudioFileOpenURL"))?;
        set_property(&mut file, CLIENT_DATA_FORMAT, &pcm_asbd(sample_rate))?;
        Ok(Self { file })
    }

    /// Reads up to `out.len()` frames; returns how many (0 at the end).
    pub(crate) fn read(&mut self, out: &mut [f32]) -> Result<usize, FinalizeError> {
        let mut frames = u32::try_from(out.len()).unwrap_or(u32::MAX);
        let mut list = buffer_list(out.as_mut_ptr(), out.len());
        self.file
            .read(&mut frames, &mut list)
            .map_err(os("ExtAudioFileRead"))?;
        Ok(usize::try_from(frames).unwrap_or_default())
    }
}

/// Encodes 48 kHz mono f32 into an AAC-LC 64 kbps file.
pub(super) struct AacWriter {
    file: ExtAudioFileRef,
}

impl AacWriter {
    /// Creates (or replaces) `path` in `container`.
    pub(super) fn create(path: &Path, container: Container) -> Result<Self, FinalizeError> {
        let file_type = match container {
            Container::M4a => audio::FileTypeId::M4A,
            Container::Mp4 => audio::FileTypeId::MPEG4,
        };
        let url = file_url(path)?;
        let flags = audio::FileFlags::ERASE_FILE;
        let mut file = ExtAudioFile::create(&url, file_type, &aac_asbd(), std::ptr::null(), flags)
            .map_err(os("ExtAudioFileCreateWithURL"))?;
        set_property(&mut file, CLIENT_DATA_FORMAT, &pcm_asbd(STORAGE_RATE))?;
        set_bit_rate(&mut file)?;
        Ok(Self { file })
    }

    /// Encodes `samples`.
    pub(super) fn write(&mut self, samples: &[f32]) -> Result<(), FinalizeError> {
        let frames = u32::try_from(samples.len()).unwrap_or(u32::MAX);
        let list = buffer_list(samples.as_ptr().cast_mut(), samples.len());
        self.file
            .write(frames, &list)
            .map_err(os("ExtAudioFileWrite"))
    }

    /// Drains the encoder and writes the trailer (`ExtAudioFileDispose`).
    pub(super) fn close(self) {
        drop(self.file);
    }
}

/// Mono f32 at `sample_rate`, the client format on both sides.
fn pcm_asbd(sample_rate: u32) -> audio::StreamBasicDesc {
    audio::StreamBasicDesc::common_f32(f64::from(sample_rate), 1, true)
}

/// 48 kHz mono AAC-LC, the file format.
fn aac_asbd() -> audio::StreamBasicDesc {
    audio::StreamBasicDesc {
        sample_rate: f64::from(STORAGE_RATE),
        format: audio::Format::MPEG4_AAC,
        frames_per_packet: u32::try_from(AAC_FRAMES_PER_PACKET).unwrap_or(1024),
        channels_per_frame: 1,
        ..Default::default()
    }
}

/// A one-buffer list over `len` mono samples at `data` (no copy).
fn buffer_list(data: *mut f32, len: usize) -> AudioBufList<1> {
    AudioBufList {
        number_buffers: 1,
        buffers: [AudioBuf {
            number_channels: 1,
            data_bytes_size: u32::try_from(len * std::mem::size_of::<f32>()).unwrap_or(u32::MAX),
            data: data.cast(),
        }],
    }
}

/// A Core Foundation file URL for `path`.
fn file_url(path: &Path) -> Result<cidre::arc::R<cf::Url>, FinalizeError> {
    cf::Url::with_path(path, false).ok_or(FinalizeError::InvalidPath)
}

/// Maps an `OSStatus` error from `call`.
fn os(call: &'static str) -> impl Fn(cidre::os::Error) -> FinalizeError {
    move |status| FinalizeError::Audio {
        call,
        status: status.0.get(),
    }
}

/// Sets the converter's bit rate and re-applies its config (Apple QA1678).
fn set_bit_rate(file: &mut ExtAudioFile) -> Result<(), FinalizeError> {
    let mut converter: *mut c_void = std::ptr::null_mut();
    get_property(file, AUDIO_CONVERTER, &mut converter)?;
    converter_set_property(converter, ENCODE_BIT_RATE, &BIT_RATE)?;
    let no_config: *const c_void = std::ptr::null();
    set_property(file, CONVERTER_CONFIG, &no_config)
}

#[allow(
    unsafe_code,
    reason = "cidre a9587fa does not bind these three AudioToolbox setters"
)]
mod ffi {
    use std::ffi::c_void;

    #[link(name = "AudioToolbox", kind = "framework")]
    unsafe extern "C" {
        pub(super) fn ExtAudioFileSetProperty(
            file: *mut c_void,
            id: u32,
            size: u32,
            data: *const c_void,
        ) -> i32;
        pub(super) fn ExtAudioFileGetProperty(
            file: *mut c_void,
            id: u32,
            size: *mut u32,
            data: *mut c_void,
        ) -> i32;
        pub(super) fn AudioConverterSetProperty(
            converter: *mut c_void,
            id: u32,
            size: u32,
            data: *const c_void,
        ) -> i32;
    }
}

/// `ExtAudioFileSetProperty` with a typed value.
#[allow(
    unsafe_code,
    reason = "FFI call with a pointer to a live, correctly sized value"
)]
fn set_property<T>(file: &mut ExtAudioFile, id: u32, value: &T) -> Result<(), FinalizeError> {
    let size = u32::try_from(std::mem::size_of::<T>()).unwrap_or(u32::MAX);
    let file: *mut ExtAudioFile = file;
    // SAFETY: `file` is a live ExtAudioFile and `value` points to `size` bytes.
    let status = unsafe {
        ffi::ExtAudioFileSetProperty(file.cast(), id, size, std::ptr::from_ref(value).cast())
    };
    check(status, "ExtAudioFileSetProperty")
}

/// `ExtAudioFileGetProperty` into a typed value.
#[allow(
    unsafe_code,
    reason = "FFI call with a pointer to a live, correctly sized value"
)]
fn get_property<T>(file: &mut ExtAudioFile, id: u32, value: &mut T) -> Result<(), FinalizeError> {
    let mut size = u32::try_from(std::mem::size_of::<T>()).unwrap_or(u32::MAX);
    let file: *mut ExtAudioFile = file;
    // SAFETY: `file` is a live ExtAudioFile and `value` has room for `size` bytes.
    let status = unsafe {
        ffi::ExtAudioFileGetProperty(
            file.cast(),
            id,
            &raw mut size,
            std::ptr::from_mut(value).cast(),
        )
    };
    check(status, "ExtAudioFileGetProperty")
}

/// `AudioConverterSetProperty` with a typed value.
#[allow(
    unsafe_code,
    reason = "FFI call with a pointer to a live, correctly sized value"
)]
fn converter_set_property<T>(
    converter: *mut c_void,
    id: u32,
    value: &T,
) -> Result<(), FinalizeError> {
    let size = u32::try_from(std::mem::size_of::<T>()).unwrap_or(u32::MAX);
    // SAFETY: `converter` comes from the live ExtAudioFile, `value` points to `size` bytes.
    let status = unsafe {
        ffi::AudioConverterSetProperty(converter, id, size, std::ptr::from_ref(value).cast())
    };
    check(status, "AudioConverterSetProperty")
}

/// Maps a raw `OSStatus`.
fn check(status: i32, call: &'static str) -> Result<(), FinalizeError> {
    if status == 0 {
        Ok(())
    } else {
        Err(FinalizeError::Audio { call, status })
    }
}
