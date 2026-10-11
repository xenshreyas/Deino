//! Opening one cpal input stream on its own thread.
//!
//! The thread builds and plays the stream, reports the stream's rate and
//! ring (or the error), then parks until told to stop and drops the stream
//! on the same thread it was built on. The caller waits at most
//! [`super::OPEN_TIMEOUT`]; a thread still stuck in the driver is abandoned
//! with its stop signal queued, so it tears down as soon as the driver returns.
//!
//! The callback is the real-time part: it converts the cpal capture instant
//! to host time against an anchor taken at the first callback, then pushes
//! the block (averaged to mono) into the preallocated capture ring. No
//! allocation, lock or log.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender, channel, sync_channel};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputCallbackInfo, SampleFormat, SizedSample, StreamInstant};

use super::OPEN_TIMEOUT;
use crate::clock::HostClock;
use crate::ring::{CaptureSample, RtSink, capture_ring};
use crate::source::{SourceError, SourceStream};
use crate::units::{HostTime, SampleRate, Track};

/// What to open.
pub(super) struct OpenRequest {
    /// The cpal device name.
    pub name: String,
    /// Host time, for the stamp anchor.
    pub clock: Arc<dyn HostClock>,
    /// Cleared by the error callback.
    pub alive: Arc<AtomicBool>,
}

/// An open stream: what it delivers, and its thread.
pub(super) struct Opened {
    pub stream: SourceStream,
    pub stop: Sender<()>,
    pub thread: JoinHandle<()>,
}

/// Opens the device on a new thread and waits for it to start.
pub(super) fn open(request: OpenRequest) -> Result<Opened, SourceError> {
    let (ready, started) = sync_channel(1);
    let (stop, stops) = channel();
    let thread = std::thread::Builder::new()
        .name("deino-audio-mic".into())
        .spawn(move || run(&request, &ready, &stops))
        .map_err(|source| SourceError::Spawn {
            track: Track::Microphone,
            source,
        })?;
    match started.recv_timeout(OPEN_TIMEOUT) {
        Ok(Ok(stream)) => Ok(Opened {
            stream,
            stop,
            thread,
        }),
        Ok(Err(error)) => {
            thread.join().ok();
            Err(error)
        }
        Err(RecvTimeoutError::Timeout) => {
            stop.send(()).ok();
            Err(SourceError::Timeout(Track::Microphone))
        }
        Err(RecvTimeoutError::Disconnected) => Err(SourceError::NoDevice(Track::Microphone)),
    }
}

/// The stream thread: build, report, wait, drop.
fn run(
    request: &OpenRequest,
    ready: &SyncSender<Result<SourceStream, SourceError>>,
    stops: &Receiver<()>,
) {
    match build(request) {
        Ok((stream, source)) => {
            ready.send(Ok(source)).ok();
            stops.recv().ok();
            drop(stream);
        }
        Err(error) => {
            ready.send(Err(error)).ok();
        }
    }
}

/// Builds and plays the named device's stream at its default config.
fn build(request: &OpenRequest) -> Result<(cpal::Stream, SourceStream), SourceError> {
    let host = cpal::default_host();
    let device = host
        .input_devices()
        .map_err(open_error)?
        .find(|device| device.name().ok().as_deref() == Some(request.name.as_str()))
        .ok_or(SourceError::NoDevice(Track::Microphone))?;
    let config = device.default_input_config().map_err(open_error)?;
    let rate = config.sample_rate().0;
    let (sink, blocks) = capture_ring(rate);
    let callback = Callback {
        sink,
        channels: usize::from(config.channels()),
        clock: Arc::clone(&request.clock),
        anchor: None,
    };
    let stream = match config.sample_format() {
        SampleFormat::F32 => build_typed::<f32>(&device, &config, callback, request)?,
        SampleFormat::I16 => build_typed::<i16>(&device, &config, callback, request)?,
        SampleFormat::I32 => build_typed::<i32>(&device, &config, callback, request)?,
        SampleFormat::F64 => build_typed::<f64>(&device, &config, callback, request)?,
        other => return Err(open_error(UnsupportedFormat(other))),
    };
    stream.play().map_err(open_error)?;
    Ok((
        stream,
        SourceStream {
            rate: SampleRate(rate),
            blocks,
        },
    ))
}

fn build_typed<S: SizedSample + CaptureSample + Send + 'static>(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    mut callback: Callback,
    request: &OpenRequest,
) -> Result<cpal::Stream, SourceError> {
    let alive = Arc::clone(&request.alive);
    device
        .build_input_stream(
            &config.config(),
            move |data: &[S], info: &InputCallbackInfo| callback.deliver(data, info),
            // The error callback may share the audio thread: only flip the flag.
            move |_error| alive.store(false, Ordering::Release),
            None,
        )
        .map_err(open_error)
}

/// The real-time callback's state.
struct Callback {
    sink: RtSink,
    channels: usize,
    clock: Arc<dyn HostClock>,
    /// The first capture instant and the host time it maps to.
    anchor: Option<(StreamInstant, HostTime)>,
}

impl Callback {
    /// Real-time safe: two atomic-free clock reads at most, no allocation,
    /// no lock, no log.
    fn deliver<S: CaptureSample>(&mut self, data: &[S], info: &InputCallbackInfo) {
        let capture = info.timestamp().capture;
        let (origin, origin_at) = *self
            .anchor
            .get_or_insert_with(|| (capture, self.clock.now()));
        let since = capture.duration_since(&origin).unwrap_or_default();
        self.sink
            .push_frames(data, self.channels, origin_at.plus(since));
    }
}

/// A sample format Deino does not convert.
#[derive(Debug, thiserror::Error)]
#[error("the microphone delivers {0:?} samples, which Deino cannot read")]
struct UnsupportedFormat(SampleFormat);

fn open_error(error: impl std::error::Error + Send + Sync + 'static) -> SourceError {
    SourceError::Open {
        track: Track::Microphone,
        source: Box::new(error),
    }
}
