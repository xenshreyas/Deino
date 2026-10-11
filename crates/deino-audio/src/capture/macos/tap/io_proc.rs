//! The tap's `IOProc`: the real-time half of system-audio capture.
//!
//! Core Audio calls [`io_proc`] on its real-time thread for every IO cycle
//! of the aggregate device. It reads the input buffer in place, converts the
//! host time of the cycle's input to nanoseconds with the timebase captured
//! at start, and pushes the block into the preallocated capture ring. No
//! allocation, no lock, no log, no system call. Adapted from anarlog's
//! `speaker/macos.rs` (MIT), without its waker (which can lock).

use cidre::{av, cat, core_audio as ca, os};

use crate::ring::{CaptureSample, RtSink};
use crate::units::HostTime;

/// Everything the `IOProc` touches; boxed and pinned for the device's life.
pub(super) struct TapContext {
    /// The capture ring's real-time end.
    pub sink: RtSink,
    /// The tap's sample format.
    pub format: av::audio::CommonFormat,
    /// Interleaved channels per frame (1 for Deino's mono tap).
    pub channels: usize,
    /// `mach_timebase_info` numerator and denominator.
    pub timebase: (u64, u64),
}

impl TapContext {
    /// Host ticks to nanoseconds (`ticks × numer / denom`).
    fn host_time(&self, ticks: u64) -> HostTime {
        let (numer, denom) = self.timebase;
        let nanos = u128::from(ticks) * u128::from(numer) / u128::from(denom.max(1));
        HostTime(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    /// Pushes one buffer of `S` samples.
    fn push<S: CaptureSample>(&mut self, buffer: &cat::AudioBuf, at: HostTime) {
        if let Some(samples) = samples::<S>(buffer) {
            self.sink.push_frames(samples, self.channels.max(1), at);
        }
    }
}

/// The `IOProc`. Real-time safe.
pub(super) extern "C" fn io_proc(
    _device: ca::Device,
    _now: &cat::AudioTimeStamp,
    input: &cat::AudioBufList<1>,
    input_time: &cat::AudioTimeStamp,
    _output: &mut cat::AudioBufList<1>,
    _output_time: &cat::AudioTimeStamp,
    context: Option<&mut TapContext>,
) -> os::Status {
    let Some(context) = context else {
        return os::Status::NO_ERR;
    };
    let buffer = &input.buffers[0];
    let at = context.host_time(input_time.host_time);
    match context.format {
        av::audio::CommonFormat::PcmF32 => context.push::<f32>(buffer, at),
        av::audio::CommonFormat::PcmF64 => context.push::<f64>(buffer, at),
        av::audio::CommonFormat::PcmI32 => context.push::<i32>(buffer, at),
        av::audio::CommonFormat::PcmI16 => context.push::<i16>(buffer, at),
        av::audio::CommonFormat::Other => {}
    }
    os::Status::NO_ERR
}

/// The buffer's bytes as `T` samples, or `None` when empty or misaligned.
#[allow(unsafe_code, reason = "reads the IOProc's input buffer in place")]
fn samples<T: Copy>(buffer: &cat::AudioBuf) -> Option<&[T]> {
    let bytes = usize::try_from(buffer.data_bytes_size).ok()?;
    let data = buffer.data.cast::<T>();
    if bytes == 0 || data.is_null() || !data.is_aligned() {
        return None;
    }
    let len = bytes / size_of::<T>();
    // SAFETY: Core Audio guarantees `data_bytes_size` readable bytes at
    // `data` for the duration of the IOProc call; the pointer is non-null
    // and aligned for `T`, and the slice does not outlive the call.
    Some(unsafe { std::slice::from_raw_parts(data, len) })
}
