//! The few C functions cidre a9587fa does not bind: the Screen Recording
//! preflight, and a pixel buffer's base address and row length. Each is a
//! plain query with no ownership transfer.

#![allow(
    unsafe_code,
    reason = "FFI to Core Graphics and Core Video functions cidre does not bind"
)]

use cidre::cv;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// Whether the process may capture the screen; never prompts (macOS 10.15+).
    fn CGPreflightScreenCaptureAccess() -> bool;
    /// Asks for Screen Recording: macOS shows its prompt the first time.
    fn CGRequestScreenCaptureAccess() -> bool;
    /// The main display; its first call connects the process to the window
    /// server, which `SCStream` asserts on in a process without `NSApplication`.
    fn CGMainDisplayID() -> u32;
}

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVPixelBufferGetBaseAddress(buffer: &cv::PixelBuf) -> *mut u8;
    fn CVPixelBufferGetBytesPerRow(buffer: &cv::PixelBuf) -> usize;
}

/// Whether Screen Recording is granted, without asking.
pub(super) fn preflight_screen_capture() -> bool {
    // SAFETY: a parameterless query.
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// Asks for Screen Recording (the macOS prompt, first time only). Only
/// ever called from the user's click in onboarding or Settings.
pub(super) fn request_screen_capture() -> bool {
    // SAFETY: a parameterless call.
    unsafe { CGRequestScreenCaptureAccess() }
}

/// Connects to the window server (a no-op once connected, as in the app).
pub(super) fn connect_window_server() {
    // SAFETY: a parameterless query.
    unsafe { CGMainDisplayID() };
}

/// The base address and row length of a non-planar pixel buffer whose base
/// address is locked; `None` when it has no memory.
pub(super) fn pixels(buffer: &cv::PixelBuf) -> Option<(*mut u8, usize)> {
    // SAFETY: plain getters on a live, locked pixel buffer.
    let (base, stride) = unsafe {
        (
            CVPixelBufferGetBaseAddress(buffer),
            CVPixelBufferGetBytesPerRow(buffer),
        )
    };
    (!base.is_null()).then_some((base, stride))
}
