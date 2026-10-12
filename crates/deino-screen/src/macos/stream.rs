//! [`MacSource`]: one window's frames through an `SCStream`.
//!
//! The stream captures the window alone (`desktopIndependentWindow`, so
//! other windows on top do not show), scaled to the size the capture asked
//! for, in 32-bit BGRA, at most `rate` frames a second. `ScreenCaptureKit`
//! calls [`FrameOutput`] on a serial dispatch queue; it copies the pixels
//! into a [`Frame`] and hands it over a two-slot channel without blocking
//! (a frame the capture thread has not taken yet is simply replaced by
//! newer ones). An unchanged window sends idle frames without pixels,
//! which are skipped. Dropping the [`MacFeed`] stops the stream.

#![allow(
    clippy::transmute_ptr_to_ptr,
    reason = "cidre's define_obj_type! and add_methods macros transmute object references"
)]
#![allow(
    unsafe_code,
    reason = "the Send wrapper for retained ScreenCaptureKit objects"
)]

use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Duration;

use cidre::sc::StreamOutput as _;
use cidre::{arc, cm, cv, define_obj_type, dispatch, ns, objc, sc};

use super::native::pixels;
use super::windows::with_content;
use crate::error::ScreenError;
use crate::frame::{Frame, FrameRate};
use crate::pick::{OWN_BUNDLE, Source};
use crate::ports::{FrameFeed, ScreenSource};

/// How long starting the stream may take.
const START_TIMEOUT: Duration = Duration::from_secs(5);

/// Window frames through `ScreenCaptureKit`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MacSource;

impl ScreenSource for MacSource {
    fn open(
        &self,
        id: u32,
        size: (u32, u32),
        rate: FrameRate,
    ) -> Result<Box<dyn FrameFeed>, ScreenError> {
        open_source(Request {
            source: Source::Window(id),
            size,
            rate,
        })
    }

    fn open_display(
        &self,
        id: u32,
        size: (u32, u32),
        rate: FrameRate,
    ) -> Result<Box<dyn FrameFeed>, ScreenError> {
        open_source(Request {
            source: Source::Display(id),
            size,
            rate,
        })
    }
}

/// Starts the stream of `request`'s window or display.
fn open_source(request: Request) -> Result<Box<dyn FrameFeed>, ScreenError> {
    let (frames, received) = sync_channel(2);
    let stream = with_content(move |content| build(content, request, frames.clone()))?
        .ok_or(ScreenError::WindowGone)?;
    start(&stream.stream)?;
    Ok(Box::new(MacFeed {
        native: stream,
        frames: received,
    }))
}

/// The filter of a window alone, or of a display without Deino's own
/// windows; `None` when it is gone.
fn filter(content: &sc::ShareableContent, source: Source) -> Option<arc::R<sc::ContentFilter>> {
    match source {
        Source::Window(id) => {
            let windows = content.windows();
            let window = windows.iter().find(|window| window.id() == id)?;
            Some(sc::ContentFilter::with_desktop_independent_window(window))
        }
        Source::Display(id) => {
            let displays = content.displays();
            let display = displays.iter().find(|shown| shown.display_id().0 == id)?;
            let windows = content.windows();
            let own: Vec<&sc::Window> = windows.iter().filter(|w| is_own(w)).collect();
            let own = ns::Array::from_slice(&own);
            Some(sc::ContentFilter::with_display_excluding_windows(
                display, &own,
            ))
        }
    }
}

/// Whether `window` is one of Deino's own.
fn is_own(window: &sc::Window) -> bool {
    window
        .owning_app()
        .is_some_and(|app| app.bundle_id().to_string() == OWN_BUNDLE)
}

/// The stream and its output, kept alive together.
struct Native {
    stream: arc::R<sc::Stream>,
    _output: arc::R<FrameOutput>,
}

// SAFETY: `SCStream` is documented thread-safe (cidre marks it Send and
// Sync); the output object is only retained here, and its inner sender is
// Send. Retain and release are atomic for every Objective-C object.
unsafe impl Send for Native {}

/// A running window capture.
struct MacFeed {
    native: Native,
    frames: Receiver<Frame>,
}

impl FrameFeed for MacFeed {
    fn next(&mut self, wait: Duration) -> Option<Frame> {
        let mut newest = self.frames.recv_timeout(wait).ok()?;
        while let Ok(frame) = self.frames.try_recv() {
            newest = frame;
        }
        Some(newest)
    }
}

impl Drop for MacFeed {
    fn drop(&mut self) {
        self.native.stream.stop_with_ch(|_| {});
    }
}

/// Which window or display to stream, at what size and rate.
#[derive(Clone, Copy, Debug)]
struct Request {
    source: Source,
    size: (u32, u32),
    rate: FrameRate,
}

/// The stream for the requested window, its output added; `None` when the
/// window is gone.
fn build(
    content: &sc::ShareableContent,
    request: Request,
    frames: SyncSender<Frame>,
) -> Option<Native> {
    let Request {
        source,
        size: (width, height),
        rate,
    } = request;
    let filter = filter(content, source)?;
    let mut cfg = sc::StreamCfg::new();
    cfg.set_width(width as usize);
    cfg.set_height(height as usize);
    cfg.set_minimum_frame_interval(cm::Time::new(1, i32::from(rate.fps())));
    cfg.set_pixel_format(cv::PixelFormat::_32_BGRA);
    cfg.set_scales_to_fit(true);
    cfg.set_shows_cursor(false);
    cfg.set_queue_depth(3);
    let stream = sc::Stream::new(&filter, &cfg);
    let output = FrameOutput::with(OutputInner { frames });
    let queue = dispatch::Queue::serial_with_ar_pool();
    stream
        .add_stream_output(output.as_ref(), sc::OutputType::Screen, Some(&queue))
        .ok()?;
    Some(Native {
        stream,
        _output: output,
    })
}

/// Starts `stream` and waits for `ScreenCaptureKit`'s answer.
fn start(stream: &sc::Stream) -> Result<(), ScreenError> {
    let (answer, answered) = sync_channel(1);
    stream.start_with_ch(move |error| {
        answer.try_send(error.is_none()).ok();
    });
    match answered.recv_timeout(START_TIMEOUT) {
        Ok(true) => Ok(()),
        Ok(false) => Err(ScreenError::Native {
            call: "SCStream startCapture",
        }),
        Err(_) => Err(ScreenError::Timeout("SCStream startCapture")),
    }
}

/// What the output holds: where frames go.
pub(crate) struct OutputInner {
    frames: SyncSender<Frame>,
}

define_obj_type!(
    pub(crate) FrameOutput + sc::StreamOutputImpl,
    OutputInner,
    DEINO_SCREEN_FRAME_OUTPUT
);

impl sc::StreamOutput for FrameOutput {}

#[objc::add_methods]
impl sc::StreamOutputImpl for FrameOutput {
    extern "C" fn impl_stream_did_output_sample_buf(
        &mut self,
        _cmd: Option<&objc::Sel>,
        _stream: &sc::Stream,
        sample_buf: &mut cm::SampleBuf,
        kind: sc::OutputType,
    ) {
        if kind != sc::OutputType::Screen {
            return;
        }
        if let Some(frame) = sample_buf.image_buf_mut().and_then(copy_frame) {
            // Full: the capture thread has frames waiting; this one goes.
            self.inner().frames.try_send(frame).ok();
        }
    }
}

/// Copies a BGRA pixel buffer into a [`Frame`].
fn copy_frame(buffer: &mut cv::PixelBuf) -> Option<Frame> {
    if buffer.pixel_format() != cv::PixelFormat::_32_BGRA {
        return None;
    }
    let width = u32::try_from(buffer.width()).ok()?;
    let height = u32::try_from(buffer.height()).ok()?;
    let lock = cv::pixel_buffer::LockFlags::READ_ONLY;
    // SAFETY: lock and unlock are paired below, on this thread.
    unsafe { buffer.lock_base_addr(lock) }.result().ok()?;
    let frame = pixels(buffer).and_then(|(base, stride)| {
        // SAFETY: the locked buffer has `height` rows of `stride` bytes at `base`.
        let data = unsafe { std::slice::from_raw_parts(base, stride * buffer.height()) };
        Frame::from_rows(width, height, stride, data).ok()
    });
    // SAFETY: the matching unlock.
    unsafe { buffer.unlock_lock_base_addr(lock) };
    frame
}
