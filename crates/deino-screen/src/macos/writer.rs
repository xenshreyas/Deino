//! [`MacEncoders`]: H.264 MP4 files through `AVAssetWriter`.
//!
//! One video input (H.264, the frame's size, a bit rate that scales with
//! the frame rate, a key frame every 10 frames so seeking stays cheap)
//! fed through a pixel buffer adaptor. Each frame is copied into a new BGRA
//! pixel buffer and stamped with its time on the recording's timeline; the
//! session starts at zero, so the file's clock is the recording's clock
//! and the player keeps it in step with `audio.mp4`. The movie is written
//! in 10 s fragments, so a crash still leaves a playable file. A frame the
//! encoder is not ready for, or one not later than the last, is skipped
//! (the next one replaces it).

#![allow(unsafe_code, reason = "pixel buffer copies and the objc2 finish call")]
#![allow(
    clippy::transmute_ptr_to_ptr,
    reason = "cidre's ns::str! macro transmutes a static CFString reference"
)]

use std::path::Path;
use std::sync::mpsc::sync_channel;
use std::time::Duration;

use block2::RcBlock;
use cidre::{arc, av, cm, cv, ns};
use deino_domain::Millis;
use objc2::msg_send;
use objc2::runtime::AnyObject;

use super::native::pixels;
use crate::error::ScreenError;
use crate::frame::{Frame, FrameRate};
use crate::ports::{VideoEncoder, VideoEncoders};

/// How long finishing the file may take.
const FINISH_TIMEOUT: Duration = Duration::from_secs(10);
/// The movie fragment length: at most this much video is lost in a crash.
const FRAGMENT_SECONDS: i64 = 10;
/// Frames between key frames.
const KEY_FRAME_INTERVAL: i32 = 10;
/// Timestamps are milliseconds.
const MILLIS_SCALE: i32 = 1_000;

/// H.264 writers through `AVAssetWriter`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MacEncoders;

impl VideoEncoders for MacEncoders {
    fn open(
        &self,
        path: &Path,
        size: (u32, u32),
        rate: FrameRate,
    ) -> Result<Box<dyn VideoEncoder>, ScreenError> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::remove_file(path).ok();
        let path = path.to_str().ok_or(ScreenError::Native {
            call: "a UTF-8 path",
        })?;
        let url = ns::Url::with_fs_path_str(path, false);
        let mut writer = av::AssetWriter::with_url_and_file_type(&url, av::FileType::mp4())
            .map_err(|_| ScreenError::Native {
                call: "AVAssetWriter init",
            })?;
        writer.set_movie_fragment_interval(cm::Time::new(FRAGMENT_SECONDS, 1));
        let input = video_input(size, rate)?;
        let adaptor = av::asset::WriterInputPixelBufAdaptor::with_input_writer(&input, None)
            .map_err(|_| ScreenError::Native {
                call: "AVAssetWriterInputPixelBufferAdaptor init",
            })?;
        writer.add_input(&input).map_err(|_| ScreenError::Native {
            call: "AVAssetWriter addInput",
        })?;
        if !writer.start_writing() {
            return Err(ScreenError::Native {
                call: "AVAssetWriter startWriting",
            });
        }
        writer.start_session_at_src_time(cm::Time::zero());
        Ok(Box::new(MacEncoder {
            native: Native {
                writer,
                input,
                adaptor,
            },
            last: None,
        }))
    }
}

/// The H.264 input for `size` at `rate`.
fn video_input(
    (width, height): (u32, u32),
    rate: FrameRate,
) -> Result<arc::R<av::AssetWriterInput>, ScreenError> {
    use av::video_settings_keys as keys;
    let compression = ns::Dictionary::<ns::String, ns::Id>::with_keys_values(
        &[
            ns::str!(c"AverageBitRate"),
            ns::str!(c"MaxKeyFrameInterval"),
            ns::str!(c"ExpectedFrameRate"),
        ],
        &[
            ns::Number::with_u32(rate.bit_rate()).as_ref(),
            ns::Number::with_i32(KEY_FRAME_INTERVAL).as_ref(),
            ns::Number::with_i32(i32::from(rate.fps())).as_ref(),
        ],
    );
    let settings = ns::Dictionary::<ns::String, ns::Id>::with_keys_values(
        &[
            keys::codec(),
            keys::width(),
            keys::height(),
            keys::compression_props(),
        ],
        &[
            av::VideoCodec::h264().as_ref(),
            ns::Number::with_u32(width).as_ref(),
            ns::Number::with_u32(height).as_ref(),
            compression.as_ref(),
        ],
    );
    let mut input = av::AssetWriterInput::with_media_type_and_output_settings(
        av::MediaType::video(),
        Some(&settings),
    )
    .map_err(|_| ScreenError::Native {
        call: "AVAssetWriterInput init",
    })?;
    input.set_expects_media_data_in_real_time(true);
    Ok(input)
}

/// The writer, its input and adaptor, kept together.
struct Native {
    writer: arc::R<av::AssetWriter>,
    input: arc::R<av::AssetWriterInput>,
    adaptor: arc::R<av::asset::WriterInputPixelBufAdaptor>,
}

// SAFETY: the writer, input and adaptor are used by one thread at a time
// (the capture thread that owns the encoder); AVFoundation allows that,
// and retain and release are atomic.
unsafe impl Send for Native {}

/// One video file being written.
struct MacEncoder {
    native: Native,
    last: Option<Millis>,
}

impl VideoEncoder for MacEncoder {
    fn append(&mut self, frame: &Frame, at: Millis) -> Result<(), ScreenError> {
        let ready = self.native.input.is_ready_for_more_media_data();
        if !ready || self.last.is_some_and(|last| at <= last) {
            return Ok(());
        }
        let buffer = pixel_buffer(frame)?;
        match self
            .native
            .adaptor
            .append_pixel_buf_with_pts(&buffer, millis(at))
        {
            Ok(true) => {
                self.last = Some(at);
                Ok(())
            }
            _ => Err(ScreenError::Native {
                call: "AVAssetWriterInputPixelBufferAdaptor append",
            }),
        }
    }

    fn finish(mut self: Box<Self>, end: Millis) -> Result<(), ScreenError> {
        self.native.input.mark_as_finished();
        let end = self.last.map_or(end, |last| end.max(last));
        self.native.writer.end_session_at_src_time(millis(end));
        finish_writing(&self.native.writer)?;
        match self.native.writer.error() {
            Some(_) => Err(ScreenError::Native {
                call: "AVAssetWriter finishWriting",
            }),
            None => Ok(()),
        }
    }
}

/// `at` as a Core Media time.
fn millis(at: Millis) -> cm::Time {
    cm::Time::new(i64::from(at.0), MILLIS_SCALE)
}

/// A new BGRA pixel buffer holding `frame`.
fn pixel_buffer(frame: &Frame) -> Result<arc::R<cv::PixelBuf>, ScreenError> {
    let failed = ScreenError::Native {
        call: "CVPixelBufferCreate",
    };
    let (width, height) = (frame.width() as usize, frame.height() as usize);
    let mut buffer =
        cv::PixelBuf::new(width, height, cv::PixelFormat::_32_BGRA, None).map_err(|_| failed)?;
    let lock = cv::pixel_buffer::LockFlags::DEFAULT;
    // SAFETY: lock and unlock are paired below, on this thread.
    unsafe { buffer.lock_base_addr(lock) }
        .result()
        .map_err(|_| ScreenError::Native {
            call: "CVPixelBufferLockBaseAddress",
        })?;
    if let Some((base, stride)) = pixels(&buffer) {
        for (row, line) in frame.bgra().chunks(frame.stride()).enumerate() {
            // SAFETY: row `row` of the locked buffer starts at `base + row ×
            // stride` and holds at least `width × 4` bytes, which `line` is.
            unsafe {
                std::ptr::copy_nonoverlapping(line.as_ptr(), base.add(row * stride), line.len())
            };
        }
    }
    // SAFETY: the matching unlock.
    unsafe { buffer.unlock_lock_base_addr(lock) };
    Ok(buffer)
}

/// `-[AVAssetWriter finishWritingWithCompletionHandler:]`, waited for.
fn finish_writing(writer: &av::AssetWriter) -> Result<(), ScreenError> {
    let (done, finished) = sync_channel(1);
    let handler = RcBlock::new(move || {
        // The receiver may have timed out and gone; nothing else to do then.
        done.try_send(()).ok();
    });
    let object: *const AnyObject = std::ptr::from_ref(writer).cast();
    // SAFETY: `writer` is a live `AVAssetWriter` (cidre's type is a
    // transparent wrapper around the object pointer); the selector takes
    // one block and returns void, and AVFoundation copies the block.
    unsafe {
        let () = msg_send![&*object, finishWritingWithCompletionHandler: &*handler];
    }
    finished
        .recv_timeout(FINISH_TIMEOUT)
        .map_err(|_| ScreenError::Timeout("AVAssetWriter finishWriting"))
}
